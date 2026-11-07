# Performance

The target is 60 fps at 1920 × 1080 on the minimum hardware. Refer to [design.md](design.md).

## Method

1. Close the other applications.
2. Build the client: `cargo build --release -p voidwright-client`.
3. Wait 15 seconds.
4. Run the benchmark: `target/release/voidwright-client --benchmark 10`.

The benchmark uses full screen, 4× MSAA, and no vertical sync. The camera turns slowly. The first second is not measured.

## Results

| Date | Commit | Hardware | Mean rate | Mean frame time |
| --- | --- | --- | --- | --- |
| 2026-11-07 | 11d0acb | Lenovo ThinkPad T590, Intel UHD Graphics 620 | 74 to 79 fps | 12.7 to 13.6 ms |

Without MSAA, the rate is approximately 10% to 15% higher.
