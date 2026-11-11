(ns glimmer-makepad.core-test
  (:require [clojure.test :refer [deftest is]]
            [glimmer.core :as ui]
            [glimmer.ratom :as r]
            [glimmer-makepad.core :as makepad]
            [glimmer-makepad.ffi :as native]))

(deftest keyed-reorders-preserve-native-identities-and-unmount-stops-reactions
  (makepad/call-with-context
    (fn [{:keys [context id] :as root}]
      (let [rows (r/atom [1 2 3])
            view (fn [] (into [:vbox] (for [n @rows] [:text-input {:key n :text (str n)}])))
            inst (ui/mount root :vbox [view])
            container (native/child-at context id 0)
            before (mapv #(native/child-at context container %) (range 3))]
        (try
          (reset! rows [3 1 2])
          (is (= [(nth before 2) (nth before 0) (nth before 1)]
                 (mapv #(native/child-at context container %) (range 3))))
          (is (= ["3" "1" "2"] (mapv #(native/read-text context (native/child-at context container %)) (range 3))))
          (finally (ui/unmount! inst root :vbox)))
        (reset! rows [4 5])
        (is (= 1 (native/node-count context)))
        (is (= 0 (native/child-count context id)))))))

(deftest failed-mount-releases-native-subtrees-and-event-handlers
  (makepad/call-with-context
    (fn [{:keys [context id] :as root}]
      (is (thrown? Exception
            (ui/mount root :vbox
              [:vbox [:button {:label "Alive briefly" :on-click (fn [] nil)}]
               [:not-a-widget]])))
      (is (= 1 (native/node-count context)))
      (is (= 0 (native/child-count context id)))
      (is (= 0 (:handlers (makepad/memory-stats)))))))

(deftest invalid-style-and-unknown-props-do-not-leak-new-widgets
  (makepad/call-with-context
    (fn [{:keys [context] :as root}]
      (doseq [hiccup [[:label {:width -10 :label "bad"}]
                     [:button {:on-click 42}]
                     [:text-input {:font-size -1}]
                     [:vbox {:nonexistent true}]]]
        (is (thrown? Exception (ui/mount root :vbox hiccup)))
        (is (= 1 (native/node-count context)))))))
