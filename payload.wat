;; Sovereign diagnostic payload — compiled to `src/wasm_payload.rs` by build.rs
(module
  (import "aether" "read_sensor" (func $read_sensor (param i32) (result i32)))
  (import "aether" "commit_uplink" (func $commit (param i32 i32)))
  (func (export "diagnostic") (result i32)
    (local $acc i32)
    (local.set $acc (call $read_sensor (i32.const 0)))
    (local.set $acc (i32.add (local.get $acc) (i32.const 0xA17E)))
    (call $commit (local.get $acc) (i32.const 0))
    (local.get $acc)
  )
)
