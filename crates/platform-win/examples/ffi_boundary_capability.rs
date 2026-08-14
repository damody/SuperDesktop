use platform_win::common::ffi_boundary::{CallbackFence, SPIKE_CALLBACK_ABI, spike_callback};

fn run(mode: usize) -> (isize, String, usize, usize) {
    let fence = CallbackFence::default();
    if mode == 3 {
        fence.begin_shutdown();
    }
    let code = unsafe { spike_callback(&fence, if mode == 3 { 0 } else { mode }) };
    let (entered, completed) = fence.counts();
    (code, format!("{:?}", fence.fatal()), entered, completed)
}

fn main() {
    let panic = run(1);
    let double = run(2);
    let shutdown = run(3);
    println!(
        "{{\"schema\":\"ffi-boundary-capability/v1\",\"abi_signature\":{:?},\"panic\":{{\"return_code\":{},\"typed_fatal\":{:?},\"entered\":{},\"completed\":{}}},\"double_callback\":{{\"return_code\":{},\"typed_fatal\":{:?},\"entered\":{},\"completed\":{}}},\"shutdown_race\":{{\"return_code\":{},\"typed_fatal\":{:?},\"entered\":{},\"completed\":{}}},\"unwind_crossed_abi\":false}}",
        SPIKE_CALLBACK_ABI,
        panic.0,
        panic.1,
        panic.2,
        panic.3,
        double.0,
        double.1,
        double.2,
        double.3,
        shutdown.0,
        shutdown.1,
        shutdown.2,
        shutdown.3
    );
}
