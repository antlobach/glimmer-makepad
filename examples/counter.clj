(ns counter
  (:require [glimmer.core :as ui]
            [glimmer.ratom :as r]
            [glimmer-makepad.core :as makepad]
            [jolt.nrepl :as nrepl]))

(defonce count* (r/atom 0))
(defonce name* (r/atom "Ada"))
(defonce rows* (r/atom [1 2 3]))
(defonce ready (promise))
(defonce updated (promise))

(defn app []
  (deliver ready true)
  (when (and (= 1 @count*) (= "Native Clojure" @name*)) (deliver updated true))
  [:vbox {:spacing 22 :padding 24 :background "#171b25" :corner-radius 20}
   [:label {:label "Glimmer Makepad" :font-size 32 :color "#c4b5fd"}]
   [:label {:label "Jolt → C ABI → GPU-rendered native widgets" :color "#a5afc3"}]
   [:text-input {:text @name* :placeholder "Your name"
                 :on-change #(reset! name* %)
                 :on-submit #(println :submitted %)}]
   [:label {:label (str "Hello, " @name* ". Count: " @count*) :font-size 22}]
   [:hbox {:spacing 12}
    [:button {:label "Increment" :on-click #(swap! count* inc)}]
    [:button {:label "Reverse rows" :background "#334155" :on-click #(swap! rows* (comp vec reverse))}]
    [:button {:label "Close" :background "#334155" :on-click makepad/close!}]]
   (into [:vbox {:spacing 8}]
         (for [n @rows*]
           [:text-input {:key n :text (str "Keyed row " n)}]))])

(defn -main [& args]
  (let [smoke? (some #{"--smoke"} args)
        stop-repl (when (some #{"--repl"} args) (nrepl/start 0))
        worker (when smoke?
                 (future
                   @ready
                   (Thread/sleep 1500)
                   (makepad/schedule!
                     #(do (reset! name* "Native Clojure")
                          (swap! count* inc)
                          (swap! rows* (comp vec reverse))
                          (println :smoke-updated (makepad/memory-stats))))
                   @updated
                   (Thread/sleep 2000)
                   (makepad/close!)))]
    (try
      (when stop-repl (println :nrepl-port (:port (meta stop-repl))))
      (ui/run app :title "Glimmer Makepad" :width 940 :height 690)
      (when worker @worker)
      (println :shutdown (makepad/memory-stats))
      (finally
        (when worker (future-cancel worker))
        (when stop-repl (stop-repl))
        (shutdown-agents)))))
