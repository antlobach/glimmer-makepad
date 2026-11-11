(ns glimmer-makepad.ffi
  "Typed declarations of the Glimmer Makepad C ABI. No callbacks or resources are global."
  (:require [jolt.ffi :as ffi]))

(ffi/defcfn abi-version "gm_abi_version" [] :uint32)
(ffi/defcfn last-error "gm_last_error" [] :string)
(ffi/defcfn context-create "gm_context_create" [] :uint64)
(ffi/defcfn context-destroy "gm_context_destroy" [:uint64] :int32)
(ffi/defcfn node-create "gm_node_create" [:uint64 :uint32] :uint64)
(ffi/defcfn node-destroy "gm_node_destroy" [:uint64 :uint64] :int32)
(ffi/defcfn child-append "gm_child_append" [:uint64 :uint64 :uint64] :int32)
(ffi/defcfn child-remove "gm_child_remove" [:uint64 :uint64 :uint64] :int32)
(ffi/defcfn child-replace "gm_child_replace" [:uint64 :uint64 :uint64 :uint64] :int32)
(ffi/defcfn child-reorder "gm_child_reorder" [:uint64 :uint64 :uint64 :uint64] :int32)
(ffi/defcfn child-count "gm_child_count" [:uint64 :uint64] :int64)
(ffi/defcfn child-at "gm_child_at" [:uint64 :uint64 :size_t] :uint64)
(ffi/defcfn node-count "gm_context_node_count" [:uint64] :int64)
(ffi/defcfn set-text "gm_node_set_text" [:uint64 :uint64 :pointer :size_t] :int32)
(ffi/defcfn copy-text "gm_node_copy_text" [:uint64 :uint64 :pointer :size_t] :int64)
(ffi/defcfn memory-live "gm_memory_live_bytes" [] :uint64)
(ffi/defcfn memory-peak "gm_memory_peak_bytes" [] :uint64)
(ffi/defcfn memory-allocations "gm_memory_allocation_count" [] :uint64)
(ffi/defcfn set-style "gm_node_set_style" [:uint64 :uint64 :pointer] :int32)
(ffi/defcfn get-style "gm_node_get_style" [:uint64 :uint64 :pointer] :int32)
(ffi/defcfn run-native "gm_run" [:uint64 :uint64 :pointer :size_t :double :double :pointer :pointer] :int32 :blocking)
(ffi/defcfn close-native "gm_close" [:uint64] :int32)
(ffi/defcfn wake "gm_wake" [] :void)
(ffi/defcfn set-placeholder "gm_node_set_placeholder" [:uint64 :uint64 :pointer :size_t] :int32)

(def style-layout
  (ffi/layout [:struct [[:width :double] [:height :double] [:spacing :double]
                       [:padding :double] [:margin :double] [:font-size :double]
                       [:corner-radius :double] [:align-x :double] [:align-y :double]
                       [:color :uint32] [:background :uint32]]]))

(defn check! [status]
  (when (neg? status)
    (throw (ex-info (last-error) {:native-status status})))
  status)

(defn handle! [handle]
  (when (zero? handle)
    (throw (ex-info (last-error) {:native-handle handle})))
  handle)

(defn write-text! [context node text]
  (with-open [arena (ffi/confined-arena)]
    (let [bytes (.getBytes (str text) "UTF-8")
          n (alength bytes)
          p (ffi/alloc arena (max 1 n))]
      (when (pos? n) (ffi/write-array p bytes))
      (check! (set-text context node p n)))))

(defn read-text [context node]
  (let [n (check! (copy-text context node ffi/null 0))]
    (if (zero? n)
      ""
      (with-open [arena (ffi/confined-arena)]
        (let [p (ffi/alloc arena n)]
          (check! (copy-text context node p n))
          (ffi/read-bytes p n))))))

(defn memory-stats []
  {:rust-live-bytes (memory-live)
   :rust-peak-bytes (memory-peak)
   :rust-allocation-count (memory-allocations)})

(defn write-style! [context node style]
  (with-open [arena (ffi/confined-arena)]
    (let [p (ffi/alloc arena style-layout)]
      (ffi/write p style-layout style)
      (check! (set-style context node p)))))

(defn write-placeholder! [context node text]
  (with-open [arena (ffi/confined-arena)]
    (let [bytes (.getBytes (str text) "UTF-8")
          n (alength bytes)
          p (ffi/alloc arena (max 1 n))]
      (when (pos? n) (ffi/write-array p bytes))
      (check! (set-placeholder context node p n)))))
