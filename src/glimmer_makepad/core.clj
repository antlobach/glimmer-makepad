(ns glimmer-makepad.core
  "Self-rendered Makepad backend. Require this namespace, then use glimmer.core/run."
  (:require [glimmer.backend :as backend]
            [glimmer-makepad.ffi :as native]
            [jolt.ffi :as ffi]))

(defonce ^:private runtime (atom nil))
(def ^:private kinds {:vbox 1 :hbox 2 :label 3 :button 4 :text-input 5})
(def ^:private style-keys
  #{:width :height :spacing :padding :margin :font-size :corner-radius
    :align-x :align-y :color :background})

(defn- current []
  (or @runtime (throw (ex-info "Makepad widgets require run or call-with-context" {}))))

(defn- color [value]
  (cond
    (and (integer? value) (<= 0 value 4294967295)) value
    (and (string? value) (re-matches #"#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?" value))
    (let [hex (subs value 1)]
      (Long/parseLong (if (= 6 (count hex)) (str hex "ff") hex) 16))
    :else (throw (ex-info "Color must be #RRGGBB, #RRGGBBAA, or an unsigned RGBA integer" {:color value}))))

(defn- dimension [value]
  (case value :fill -1.0 :fit -2.0
    (if (and (number? value) (<= 0 value 3.4028234663852886E38))
      (double value)
      (throw (ex-info "Size must be :fill, :fit, or finite nonnegative logical pixels" {:size value})))))

(defn- style [tag props]
  (let [control? (contains? #{:button :text-input} tag)
        base {:width (if (contains? #{:label :button} tag) :fit :fill) :height :fit
              :spacing 12 :padding (if control? 12 0) :margin 0 :font-size 16
              :corner-radius (if control? 10 0) :align-x 0 :align-y 0
              :color "#e8eaf0"
              :background (case tag :button "#7255e8" :text-input "#20242e" "#00000000")}
        values (reduce
                 (fn [values key]
                   (let [value (get values key)]
                     (when-not (number? value)
                       (throw (ex-info "Style metrics must be numeric" {:property key :value value})))
                     (assoc values key (double value))))
                 (merge base (select-keys props style-keys))
                 [:spacing :padding :margin :font-size :corner-radius :align-x :align-y])]
    (-> values
        (update :width dimension)
        (update :height dimension)
        (update :color color)
        (update :background color))))

(defn- validate-props! [tag props]
  (when-not (contains? kinds tag) (throw (ex-info "Unsupported Makepad widget" {:tag tag})))
  (let [allowed (into style-keys (case tag
                                 :label [:key :label]
                                 :button [:key :label :on-click]
                                 :text-input [:key :text :placeholder :on-change :on-submit]
                                 [:key]))
        unknown (seq (remove allowed (keys props)))]
    (when unknown (throw (ex-info "Unsupported Makepad properties" {:tag tag :properties unknown}))))
  (when (and (= tag :label) (or (contains? props :background) (contains? props :corner-radius)))
    (throw (ex-info "Label backgrounds belong on their containing box" {:tag tag})))
  (doseq [key [:on-click :on-change :on-submit]]
    (when-let [handler (get props key)]
      (when-not (ifn? handler)
        (throw (ex-info "Event handler must be callable" {:tag tag :property key})))))
  (style tag props))

(defn- write-props! [tag widget props values]
  (let [{:keys [context id]} widget
        handlers (into {} (filter (comp some? val) (select-keys props [:on-click :on-change :on-submit])))]
    (native/write-style! context id values)
    (when (contains? #{:label :button :text-input} tag)
      (native/write-text! context id (get props (if (= tag :text-input) :text :label) "")))
    (when (= tag :text-input)
      (native/write-placeholder! context id (get props :placeholder "")))
    (swap! (:handlers (current)) #(if (seq handlers) (assoc % id handlers) (dissoc % id)))
    widget))

(defn- apply-props! [tag widget props]
  (write-props! tag widget props (validate-props! tag props)))

(defn- create! [tag props]
  (let [values (validate-props! tag props)
        ctx (:context (current))
        id (native/handle! (native/node-create ctx (get kinds tag)))
        widget {:context ctx :id id :tag tag}]
    (try
      (write-props! tag widget props values)
      (catch Throwable error
        (native/node-destroy ctx id)
        (throw error)))))

(defn- subtree-ids [context root]
  (loop [pending [root] found []]
    (if-let [id (peek pending)]
      (let [n (native/check! (native/child-count context id))
            children (mapv #(native/child-at context id %) (range n))]
        (recur (into (pop pending) children) (conj found id)))
      found)))

(defn- forget-handlers! [ids]
  (swap! (:handlers (current)) #(apply dissoc % ids)))

(defn- remove-child! [_ parent child]
  (let [ctx (:context parent) ids (subtree-ids ctx (:id child))]
    (native/check! (native/child-remove ctx (:id parent) (:id child)))
    (forget-handlers! ids)))

(defn- replace-child! [_ parent old new]
  (let [ctx (:context parent) ids (subtree-ids ctx (:id old))]
    (native/check! (native/child-replace ctx (:id parent) (:id old) (:id new)))
    (forget-handlers! ids)))

(defn- dispose! [widget]
  (let [ctx (:context widget) ids (subtree-ids ctx (:id widget))]
    (native/check! (native/node-destroy ctx (:id widget)))
    (forget-handlers! ids)))

(defn- drain! [rt]
  (let [tasks (first (swap-vals! (:queue rt) (constantly [])))]
    (doseq [task tasks] (task))))

(defn schedule!
  "Queue a thunk for the GUI thread. Safe from a worker or an nREPL thread."
  [work]
  (let [rt (current)]
    (when-not @(:closed? rt)
      (swap! (:queue rt) conj work)
      (native/wake)))
  nil)

(defn close!
  "Request application shutdown. After GUI startup, closing also ends Jolt."
  []
  (let [rt (current)]
    (schedule! #(native/check! (native/close-native (:context rt)))))
  nil)

(defn memory-stats
  "Rust heap counters, plus retained widget/handler counts during an active run."
  []
  (merge (native/memory-stats)
         (when-let [rt @runtime]
           {:widgets (native/check! (native/node-count (:context rt)))
            :handlers (count @(:handlers rt))})))

(defn call-with-context
  "Construct and inspect a headless native widget tree. f receives the root widget.
   Resources are confined to the caller's OS thread and released on return."
  [f]
  (let [ctx (native/handle! (native/context-create))
        rt {:context ctx :handlers (atom {}) :queue (atom []) :closed? (atom false)}]
    (if-not (compare-and-set! runtime nil rt)
      (do (native/context-destroy ctx) (throw (ex-info "A Makepad context is already active" {})))
      (try
        (when-not (= 2 (native/abi-version)) (throw (ex-info "Unsupported Makepad ABI version" {})))
        (f {:context ctx :id (native/handle! (native/node-create ctx 1)) :tag :vbox})
        (finally
          (reset! (:closed? rt) true)
          (reset! runtime nil)
          (native/check! (native/context-destroy ctx)))))))

(defn- run-lifecycle [opts lifecycle]
  (call-with-context
    (fn [root]
      (let [rt (current)
            title (.getBytes (str (get opts :title "Glimmer Makepad")) "UTF-8")]
        (with-open [arena (ffi/confined-arena)]
          (let [p (ffi/alloc arena (max 1 (alength title)))
                callback (ffi/callback arena
                           (fn [_ node event _]
                             (try
                               (case event
                                 1 ((:mount! lifecycle) root :vbox)
                                 4 (do
                                     (reset! (:closed? rt) true)
                                     (reset! (:queue rt) [])
                                     (reset! backend/loop-running? false)
                                     ((:unmount! lifecycle)))
                                 5 (drain! rt)
                                 2 (when-let [handler (get-in @(:handlers rt) [node :on-click])] (handler))
                                 3 (when-let [handler (get-in @(:handlers rt) [node :on-change])]
                                     (handler (native/read-text (:context rt) node)))
                                 6 (when-let [handler (get-in @(:handlers rt) [node :on-submit])]
                                     (handler (native/read-text (:context rt) node)))
                                 (throw (ex-info "Unknown Makepad event" {:event event :node node})))
                               0
                               (catch Throwable error
                                 (binding [*out* *err*]
                                   (println "Glimmer Makepad callback failed:" error))
                                 1)))
                           [:uint64 :uint64 :uint32 :pointer] :int32 :collect-safe)]
            (when (pos? (alength title)) (ffi/write-array p title))
            (reset! backend/loop-running? true)
            (try
              (let [result (native/run-native (:context root) (:id root) p (alength title)
                                             (double (get opts :width 900)) (double (get opts :height 640))
                                             callback ffi/null)]
                (native/check! result))
              (finally
                (reset! backend/loop-running? false)
                ((:unmount! lifecycle))))))))))

(def backend
  {:name :makepad
   :create! create!
   :apply-props! apply-props!
   :append-child! (fn [_ p c] (native/check! (native/child-append (:context p) (:id p) (:id c))))
   :remove-child! remove-child!
   :replace-child! replace-child!
   :reorder-child! (fn [_ p c previous] (native/check! (native/child-reorder (:context p) (:id p) (:id c) (or (:id previous) 0))))
   :dispose! dispose!
   :schedule schedule!
   :run-lifecycle run-lifecycle})

(backend/register! backend)
