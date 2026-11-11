(ns glimmer-makepad.ffi-test
  (:require [clojure.test :refer [deftest is testing]]
            [glimmer-makepad.ffi :as native]))

(deftest text-round-trip-and-subtree-lifetime
  (let [ctx (native/handle! (native/context-create))]
    (try
      (let [parent (native/handle! (native/node-create ctx 1))
            label (native/handle! (native/node-create ctx 3))
            text "Glimmer → Rust · 日本語 · 😀\u0000tail"]
        (native/write-text! ctx label text)
        (is (= text (native/read-text ctx label)))
        (native/check! (native/child-append ctx parent label))
        (is (thrown? Exception (native/check! (native/child-append ctx label parent))))
        (is (= label (native/child-at ctx parent 0)))
        (native/check! (native/node-destroy ctx parent))
        (is (= 0 (native/node-count ctx)))
        (is (thrown? Exception (native/read-text ctx label))))
      (finally (native/check! (native/context-destroy ctx))))))
