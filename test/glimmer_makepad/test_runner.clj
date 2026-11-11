(ns glimmer-makepad.test-runner
  (:require [clojure.test :as test]
            [glimmer-makepad.ffi-test]
            [glimmer-makepad.core-test]))

(defn -main [& _]
  (let [{:keys [fail error]} (test/run-tests 'glimmer-makepad.ffi-test 'glimmer-makepad.core-test)]
    (when (pos? (+ fail error)) (System/exit 1))))
