
#[cfg(feature = "arbitrary")]
mod fuzz {
    use arbitrary::{Arbitrary, Unstructured};
    use visual_novel_engine::{ResourceLimiter, ScriptRaw, SecurityPolicy};

    #[test]
    fn fuzz_compile_raw_scripts() {
        // Use a deterministic seed for reproducibility in this basic test
        let seed = b"fuzz test seed 1234567890";
        let mut raw_data = [0u8; 1024 * 64]; // 64KB buffer
        // Fill data with pseudo-random (deterministic) bytes or just use the seed repeated?
        // Let's make a simple linear congruential generator or similar for variety if needed,
        // but for a single run, just patterns is okay.
        // Actually, let's try multiple iterations with different seeds/data.
        
        for i in 0..100 {
            // Fill buffer with something varying
            for (j, byte) in raw_data.iter_mut().enumerate() {
                *byte = ((i * j) & 0xFF) as u8;
            }

            let mut u = Unstructured::new(&raw_data);
            
            // Attempt to generate a ScriptRaw
            if let Ok(script) = ScriptRaw::arbitrary(&mut u) {
                // It generated "successfully" structurally (strings are valid utf8 etc).
                // Now try to compile/validate it.
                // It SHOULD fail validation most of the time (missing labels, resource limits),
                // but it MUST NOT PANIC.
                
                let policy = SecurityPolicy::default();
                let limits = ResourceLimiter::default(); // Limits might be hit
                
                // 1. Validation check
                let _ = policy.validate(&script, limits);

                // 2. Compilation check
                let _ = script.compile();
            }
        }
    }
}
