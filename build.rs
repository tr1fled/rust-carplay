use std::path::PathBuf;
use std::env;

fn main() {
    // Re-run if any C source or header changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=c");
    
    // OpenSSL linking
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
    
    // Determine include directories
    let include_dirs = vec![
    "c/Support",
    "c/External/Curve25519",
    "c/External/Ed25519",
    "c/External/LibTomMath",
    "c/External/SRP",
    "c/Sources",
    "c/ChaChaCryptor",
    ];
    
    let include_dirs: Vec<PathBuf> = include_dirs
    .into_iter()
    .map(PathBuf::from)
    .collect();
    
    // Build C sources
    let mut build = cc::Build::new();
    
    // List all your C files here
    build.files([
        "c/External/LibTomMath/bncore.c",
        "c/External/LibTomMath/bn_fast_mp_invmod.c",
        "c/External/LibTomMath/bn_fast_mp_montgomery_reduce.c",
        "c/External/LibTomMath/bn_fast_s_mp_mul_digs.c",
        "c/External/LibTomMath/bn_fast_s_mp_mul_high_digs.c",
        "c/External/LibTomMath/bn_fast_s_mp_sqr.c",
        "c/External/LibTomMath/bn_mp_2expt.c",
        "c/External/LibTomMath/bn_mp_abs.c",
        "c/External/LibTomMath/bn_mp_add.c",
        "c/External/LibTomMath/bn_mp_add_d.c",
        "c/External/LibTomMath/bn_mp_clamp.c",
        "c/External/LibTomMath/bn_mp_clear.c",
        "c/External/LibTomMath/bn_mp_clear_multi.c",
        "c/External/LibTomMath/bn_mp_cmp.c",
        "c/External/LibTomMath/bn_mp_cmp_d.c",
        "c/External/LibTomMath/bn_mp_cmp_mag.c",
        "c/External/LibTomMath/bn_mp_cnt_lsb.c",
        "c/External/LibTomMath/bn_mp_copy.c",
        "c/External/LibTomMath/bn_mp_count_bits.c",
        "c/External/LibTomMath/bn_mp_div_2.c",
        "c/External/LibTomMath/bn_mp_div_2d.c",
        "c/External/LibTomMath/bn_mp_div_3.c",
        "c/External/LibTomMath/bn_mp_div.c",
        "c/External/LibTomMath/bn_mp_div_d.c",
        "c/External/LibTomMath/bn_mp_dr_is_modulus.c",
        "c/External/LibTomMath/bn_mp_dr_reduce.c",
        "c/External/LibTomMath/bn_mp_dr_setup.c",
        "c/External/LibTomMath/bn_mp_exch.c",
        "c/External/LibTomMath/bn_mp_exptmod.c",
        "c/External/LibTomMath/bn_mp_exptmod_fast.c",
        "c/External/LibTomMath/bn_mp_grow.c",
        "c/External/LibTomMath/bn_mp_init.c",
        "c/External/LibTomMath/bn_mp_init_copy.c",
        "c/External/LibTomMath/bn_mp_init_multi.c",
        "c/External/LibTomMath/bn_mp_init_size.c",
        "c/External/LibTomMath/bn_mp_invmod.c",
        "c/External/LibTomMath/bn_mp_invmod_slow.c",
        "c/External/LibTomMath/bn_mp_karatsuba_mul.c",
        "c/External/LibTomMath/bn_mp_karatsuba_sqr.c",
        "c/External/LibTomMath/bn_mp_lshd.c",
        "c/External/LibTomMath/bn_mp_mod_2d.c",
        "c/External/LibTomMath/bn_mp_mod.c",
        "c/External/LibTomMath/bn_mp_mod_d.c",
        "c/External/LibTomMath/bn_mp_montgomery_calc_normalization.c",
        "c/External/LibTomMath/bn_mp_montgomery_reduce.c",
        "c/External/LibTomMath/bn_mp_montgomery_setup.c",
        "c/External/LibTomMath/bn_mp_mul_2.c",
        "c/External/LibTomMath/bn_mp_mul_2d.c",
        "c/External/LibTomMath/bn_mp_mul.c",
        "c/External/LibTomMath/bn_mp_mul_d.c",
        "c/External/LibTomMath/bn_mp_mulmod.c",
        "c/External/LibTomMath/bn_mp_prime_is_divisible.c",
        "c/External/LibTomMath/bn_mp_prime_is_prime.c",
        "c/External/LibTomMath/bn_mp_prime_miller_rabin.c",
        "c/External/LibTomMath/bn_mp_prime_rabin_miller_trials.c",
        "c/External/LibTomMath/bn_mp_read_unsigned_bin.c",
        "c/External/LibTomMath/bn_mp_reduce_2k.c",
        "c/External/LibTomMath/bn_mp_reduce_2k_l.c",
        "c/External/LibTomMath/bn_mp_reduce_2k_setup.c",
        "c/External/LibTomMath/bn_mp_reduce_2k_setup_l.c",
        "c/External/LibTomMath/bn_mp_reduce.c",
        "c/External/LibTomMath/bn_mp_reduce_is_2k.c",
        "c/External/LibTomMath/bn_mp_reduce_is_2k_l.c",
        "c/External/LibTomMath/bn_mp_reduce_setup.c",
        "c/External/LibTomMath/bn_mp_rshd.c",
        "c/External/LibTomMath/bn_mp_set.c",
        "c/External/LibTomMath/bn_mp_set_int.c",
        "c/External/LibTomMath/bn_mp_sqr.c",
        "c/External/LibTomMath/bn_mp_sqrmod.c",
        "c/External/LibTomMath/bn_mp_sub.c",
        "c/External/LibTomMath/bn_mp_sub_d.c",
        "c/External/LibTomMath/bn_mp_toom_mul.c",
        "c/External/LibTomMath/bn_mp_toom_sqr.c",
        "c/External/LibTomMath/bn_mp_to_unsigned_bin.c",
        "c/External/LibTomMath/bn_mp_unsigned_bin_size.c",
        "c/External/LibTomMath/bn_mp_zero.c",
        "c/External/LibTomMath/bn_prime_tab.c",
        "c/External/LibTomMath/bn_reverse.c",
        "c/External/LibTomMath/bn_s_mp_add.c",
        "c/External/LibTomMath/bn_s_mp_exptmod.c",
        "c/External/LibTomMath/bn_s_mp_mul_digs.c",
        "c/External/LibTomMath/bn_s_mp_mul_high_digs.c",
        "c/External/LibTomMath/bn_s_mp_sqr.c",
        "c/External/LibTomMath/bn_s_mp_sub.c",
        "c/External/SRP/cstr.c",
        "c/External/SRP/srp6_client.c",
        "c/External/SRP/srp6_server.c",
        "c/External/SRP/srp.c",
        "c/External/SRP/t_math.c",
        "c/Support/AESUtils.c",
        "c/Support/AsyncConnection.c",
        "c/Support/BonjourBrowser.c",
        "c/Support/CFCompat.c",
        "c/Support/CFLite.c",
        "c/Support/CFLiteBinaryPlist.c",
        "c/Support/CFLitePreferencesFile.c",
        "c/Support/CFLiteRunLoopSelect.c",
        "c/Support/CFUtils.c",
        "c/Support/ChaCha20Poly1305.c",
        "c/Support/CommandLineUtils.c",
        "c/External/Curve25519/curve25519-donna.c",
        "c/Support/DataBufferUtils.c",
        "c/Support/DebugIPCUtils.c",
        "c/Support/DebugServices.c",
        "c/Support/DispatchLite.c",
        "c/External/Ed25519/ed25519.c",
        "c/Support/HTTPClient.c",
        "c/Support/HTTPMessage.c",
        "c/Support/HTTPServer.c",
        "c/Support/HTTPUtils.c",
        "c/Support/KeychainLiteFile.c",
        "c/Support/KeychainUtils.c",
        "c/Support/LogUtils.c",
        "c/Support/MathUtils.c",
        "c/Support/MFiSAP.c",
        "c/Support/MFiServerPlatformLinux.c",
        "c/Support/MiscUtils.c",
        "c/Support/NetTransportChaCha20Poly1305.c",
        "c/Support/NetUtils.c",
        "c/Support/NetworkChangeListener.c",
        "c/Support/PairingUtils.c",
        "c/Support/PrintFUtils.c",
        "c/Support/RandomNumberUtils.c",
        "c/Support/SHAUtils.c",
        "c/Support/SRPUtils.c",
        "c/Support/StringUtils.c",
        "c/Support/ThreadUtils.c",
        "c/Support/TickUtils.c",
        "c/Support/TimeUtils.c",
        "c/Support/TLVUtils.c",
        "c/Support/URLUtils.c",
        "c/Support/utfconv.c",
        "c/Support/UUIDUtils.c",
        "c/Sources/AirTunesClock.c",
        "c/ChaChaCryptor/cryptor.c"
        ]);
        
        // Build the bindings
        /*let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
        
        // Add include paths from pkg-config
        for path in &include_dirs {
            builder = builder.clang_arg(format!("-I{}", path.display()));
        }
        
        // Generate bindings
        let bindings = builder
        .generate()
        .expect("Unable to generate OpenSSL SHA bindings");
        
        // Write to $OUT_DIR/bindings.rs
        let out_path = PathBuf::from("/home/poc/Desktop/rustcp/src/chachacryptor");
        bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");*/
        
        build
        .warnings(true)
        .extra_warnings(true)
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wextra")
        .flag_if_supported("-fPIC")
        .define("_GNU_SOURCE", Some("1"))
        .define("FRAMEWORK_STYLE_INCLUDES", Some("0"))
        .define("TARGET_HAS_NATIVE_INT128", Some("0"))
        .define("TARGET_NEEDS_NATURAL_ALIGNMENT", Some("1"))
        .define("DEBUG", Some("1"))
        .define("TARGET_HAS_LIBTOMMATH", Some("1"))
        .define("AES_UTILS_HAS_GLADMAN_GCM", Some("0"))
        .define("COMMON_SERVICES_NO_CORE_SERVICES", Some("1"))
        .define("LOGUTILS_CF_ENABLED", Some("1"))
        .define("CFLITE_ENABLED", Some("1"))
        .define("CLI_HAS_INTERACTIVE", Some("0"))
        .define("TARGET_HAS_COMMON_CRYPTO", Some("0"))
        .define("TARGET_NO_OPENSSL", Some("0")) // <-- enable OpenSSL
        .define("OPENSSL_API_COMPAT", Some("0x10101000L"));
        
        for dir in &include_dirs {
            build.include(dir);
        }
        
        build.compile("CoreUtils");
        
        // OpenSSL linking must come *after* CoreUtils
        println!("cargo:rustc-link-lib=ssl");
        println!("cargo:rustc-link-lib=crypto");
        
        // On Linux, link dns_sd if needed
        #[cfg(target_os = "linux")]
        println!("cargo:rustc-link-lib=dns_sd");
        
    }
    