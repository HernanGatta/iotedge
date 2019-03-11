// Copyright (c) Microsoft. All rights reserved.
extern crate cmake;

use std::fs;
use std::path::PathBuf;

use std::env;
use std::path::Path;
use std::process::Command;

use cmake::Config;

const SSL_OPTION: &str = "use_openssl";
const USE_EMULATOR: &str = "use_emulator";

trait SetPlatformDefines {
    fn set_platform_defines(&mut self) -> &mut Self;
    fn set_build_shared(&mut self) -> &mut Self;
}

trait SetEnclaveDefines {
    fn set_enclave_options(&mut self) -> &mut Self;
}

impl SetPlatformDefines for Config {
    #[cfg(windows)]
    fn set_platform_defines(&mut self) -> &mut Self {
        // if the builder chooses to set "use_emulator", use their setting, otherwise, use the
        // emulator for debug and a real device for release
        let use_emulator = env::var(USE_EMULATOR)
            .or_else(|_| {
                env::var("PROFILE").and_then(|profile| {
                    Ok(if profile.to_lowercase() == "release" {
                        String::from("OFF")
                    } else {
                        String::from("ON")
                    })
                })
            })
            .unwrap();
        // C-shared library wants Windows flags (/DWIN32 /D_WINDOWS) for Windows,
        // and the cmake library overrides this.
        self.cflag("/DWIN32")
            .cxxflag("/DWIN32")
            .cflag("/D_WINDOWS")
            .cxxflag("/D_WINDOWS")
            .build_arg("/m")
            .define(USE_EMULATOR, use_emulator)
            .define("use_cppunittest", "OFF")
    }

    #[cfg(unix)]
    fn set_platform_defines(&mut self) -> &mut Self {
        let rv = if env::var("TARGET").unwrap().starts_with("x86_64")
            && env::var("RUN_VALGRIND").is_ok()
        {
            "ON"
        } else {
            "OFF"
        };
        if let Ok(sysroot) = env::var("SYSROOT") {
            self.define("run_valgrind", rv)
                .define("CMAKE_SYSROOT", sysroot)
                .define(USE_EMULATOR, "OFF")
        } else {
            self.define("run_valgrind", rv).define(USE_EMULATOR, "OFF")
        }
    }

    // The "debug_assertions" configuration flag seems to be the way to detect
    // if this is a "dev" build or any other kind of build.
    #[cfg(debug_assertions)]
    fn set_build_shared(&mut self) -> &mut Self {
        self.define("BUILD_SHARED", "OFF")
    }

    #[cfg(not(debug_assertions))]
    fn set_build_shared(&mut self) -> &mut Self {
        self.define("BUILD_SHARED", "ON")
    }
}

impl SetEnclaveDefines for Config {
    #[cfg(windows)]
    fn set_enclave_options(&mut self) -> &mut Self {
        if env::var("USE_ENCLAVE").is_ok() {
            let tee = env::var("USE_ENCLAVE").unwrap().to_lowercase();
            let use_simulation = env::var("USE_SIMULATION").is_ok();

            if tee == "intel sgx" || tee == "sgx" {
                self.define("OE_TEE", "SGX")
                    .define("use_enclave", "ON");

                if use_simulation {
                    self.define("OE_USE_SIMULATION", "ON")
                } else {
                    self
                }
            } else {
                panic!("Building the HSM enclave on Windows is currently only supported for Intel SGX");
            }
        } else {
            self
        }
    }

    #[cfg(unix)]
    fn set_enclave_options(&mut self) -> &mut Self {
        if env::var("USE_ENCLAVE").is_ok() {
            let tee = env::var("USE_ENCLAVE").unwrap().to_lowercase();
            let use_simulation = env::var("USE_SIMULATION").is_ok();

            if tee == "arm trustzone" || tee == "tz" {
                if use_simulation {
                    panic!("Simulation builds are not yet supported for ARM TrustZone on Linux");
                }

                let ta_dev_kit_path = PathBuf::from("azure-iot-hsm-c/deps/optee/ta_dev_kit");
                let ta_dev_kit_abs_path = fs::canonicalize(&ta_dev_kit_path);

                self.define("OE_TEE", "TZ")
                    .define("TA_DEV_KIT_DIR", ta_dev_kit_abs_path.unwrap().to_str().unwrap())
                    .define("use_enclave", "ON")
            } else {
                panic!("Building the HSM enclave on Linux is currently only supported for ARM TrustZone");
            }
        } else {
            self
        }
    }
}

fn main() {
    // Clone Azure C -shared library
    let c_shared_repo = "azure-iot-hsm-c/deps/c-shared";
    let utpm_repo = "azure-iot-hsm-c/deps/utpm";
    let oe_repo = "azure-iot-hsm-c/deps/openenclave";
    let oe_new_platforms = format!("{}/new_platforms", oe_repo);

    let use_enclave = if env::var("USE_ENCLAVE").is_ok() {
        true
    } else {
        false
    };

    println!("#Start Update C-Shared Utilities");
    if !Path::new(&format!("{}/.git", c_shared_repo)).exists()
        || !Path::new(&format!("{}/.git", utpm_repo)).exists()
        || (use_enclave && !Path::new(&format!("{}/.git", oe_repo)).exists())
    {
        let _ = Command::new("git")
            .arg("submodule")
            .arg("update")
            .arg("--init")
            .arg("--recursive")
            .status()
            .expect("submodule update failed");
    }

    println!("#Done Updating C-Shared Utilities");

    println!("#Start building shared utilities");
    let _shared = Config::new(c_shared_repo)
        .define(SSL_OPTION, "ON")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("run_unittests", "OFF")
        .define("use_default_uuid", "ON")
        .define("use_http", "OFF")
        .define("skip_samples", "ON")
        .set_platform_defines()
        .define("run_valgrind", "OFF")
        .profile("Release")
        .build();

    println!("#Also build micro tpm library");
    let _shared = Config::new(utpm_repo)
        .define(SSL_OPTION, "ON")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("run_unittests", "OFF")
        .define("use_default_uuid", "ON")
        .define("use_http", "OFF")
        .define("skip_samples", "ON")
        .set_platform_defines()
        .define("run_valgrind", "OFF")
        .profile("Release")
        .build();

    if use_enclave {
        #[cfg(unix)]
        {
            // Using an enclave on Unix currently implies building for ARM and
            // TrustZone as the Trusted Execution Technology (TEE). As such,
            // this build script will execute inside the cross-compilation
            // container that Cross spawns during build.
            println!("#Install PIP");
            Command::new("apt-get")
                .arg("install")
                .arg("python-pip")
                .arg("python-dev")
                .arg("libgmp3-dev")
                .arg("-y")
                .status()
                .expect("apt-get install failed");

            println!("#Install PyCrypto");
            Command::new("pip")
                .arg("install")
                .arg("-i")
                .arg("https://pypi.python.org/simple/")
                .arg("pycrypto")
                .current_dir("/target")
                .status()
                .expect("pip install pycrypto failed");
        }

        println!("#And build the Open Enclave SDK");
        let _shared = Config::new(oe_new_platforms)
            .set_enclave_options()
            .set_platform_defines()
            .profile("Release")
            .build();
    }

    // make the C libary at azure-iot-hsm-c (currently a subdirectory in this
    // crate)
    // Always make the Release version because Rust links to the Release CRT.
    // (This is especially important for Windows)

    let rut = if env::var("FORCE_NO_UNITTEST").is_ok() {
        "OFF"
    } else {
        "ON"
    };

    println!("#Start building HSM dev-mode library");
    let iothsm = Config::new("azure-iot-hsm-c")
        .define(SSL_OPTION, "ON")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("run_unittests", rut)
        .define("use_default_uuid", "ON")
        .define("use_http", "OFF")
        .define("skip_samples", "ON")
        .set_enclave_options()
        .set_platform_defines()
        .set_build_shared()
        .profile("Release")
        .build();

    println!("#Done building HSM dev-mode library");

    // where to find the library (The "link-lib" should match the library name
    // defined in the CMakefile.txt)

    println!("cargo:rerun-if-env-changed=RUN_VALGRIND");
    // For libraries which will just install in target directory
    println!("cargo:rustc-link-search=native={}", iothsm.display());
    // For libraries (ie. C Shared) which will install in $target/lib
    println!("cargo:rustc-link-search=native={}/lib", iothsm.display());
    println!("cargo:rustc-link-search=native={}/lib64", iothsm.display());
    println!("cargo:rustc-link-lib=iothsm");

    // we need to explicitly link with c shared util only when we build the C
    // library as a static lib which we do only in rust debug builds
    #[cfg(debug_assertions)]
    println!("cargo:rustc-link-lib=aziotsharedutil");
    #[cfg(debug_assertions)]
    println!("cargo:rustc-link-lib=utpm");

    if use_enclave {
        println!("cargo:rustc-link-lib=oesocket_host");
        println!("cargo:rustc-link-lib=oestdio_host");
        println!("cargo:rustc-link-lib=oehost");

        #[cfg(unix)]
        println!("cargo:rustc-link-lib=teec");
    }

    #[cfg(windows)]
    {
        println!(
            "cargo:rustc-link-search=native={}/lib",
            env::var("OPENSSL_ROOT_DIR").unwrap()
        );
        println!("cargo:rustc-link-lib=libeay32");
        println!("cargo:rustc-link-lib=ssleay32");

        println!(
            "cargo:rustc-link-search=native={}/bin/x64/Release",
            env::var("SGXSDKInstallPath").unwrap()
        );

        if use_enclave {
            if fs::copy(
                format!("{}/bin/enc.signed.dll", iothsm.display()),
                format!("{}/../../../deps/enc.signed.dll", iothsm.display())).is_err() {
                panic!("Failed to copy enclave to output directory");
            }

            println!("cargo:rustc-link-lib=sgx_urts_sim");
            println!("cargo:rustc-link-lib=sgx_uae_service_sim");
            println!("cargo:rustc-link-lib=sgx_uprotected_fs");
        }
    }

    #[cfg(unix)]
    println!("cargo:rustc-link-lib=crypto");
}
