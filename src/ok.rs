use anyhow::{anyhow, Result};
use colored::Colorize;
use msru::{Accessor, Msr};
use std::process::Command;

#[derive(Debug, Default)]
enum TestState {
    Ok,
    #[default]
    Fail,
    #[allow(dead_code)]
    Warning,
    Tbd,
    Skip,
}

impl From<&TestState> for String {
    fn from(res: &TestState) -> Self {
        match res {
            TestState::Ok => "OK".to_string(),
            TestState::Fail => "FAIL".to_string(),
            TestState::Warning => "WARNING".to_string(),
            TestState::Tbd => "TBD".to_string(),
            TestState::Skip => "SKIP".to_string(),
        }
    }
}

#[derive(Debug, Default)]
enum TestOptionalState {
    #[default]
    Required,
    Optional,
}

#[derive(Debug, Default)]
enum TestOperationState {
    Manual,
    #[default]
    Program,
}

#[derive(Debug)]
enum KvmParameter {
    Tdx,
    Sgx,
}

#[derive(Debug, Default)]
struct TestResult {
    action: String,
    reason: String,
    state: TestState,
    optional_state: TestOptionalState,
    operation: TestOperationState,
}

struct Test {
    name: &'static str,
    run: Box<dyn Fn() -> TestResult>,
    sub_tests: Vec<Test>,
    post_run: Option<Box<dyn Fn()>>,
}

const SUPPORTED_OSES: [&str; 3] = [
    "Ubuntu 22.04.1 LTS",
    "Red Hat Enterprise Linux 8.7 (Ootpa)",
    "CentOS Stream 9",
];

fn get_os_pretty_name() -> String {
    let os_release =
        std::fs::read_to_string("/etc/os-release").expect("/etc/os-release does not exist");
    let pretty_name_line = os_release
        .lines()
        .find(|l| l.contains("PRETTY_NAME"))
        .expect("PRETTY_NAME for os-release does not exist");
    pretty_name_line[pretty_name_line
        .find('"')
        .expect("\" character not found in this line")..]
        .trim_matches('\"')
        .to_owned()
}

fn check_os() -> bool {
    // get os name
    let pretty_name = get_os_pretty_name();

    // check if the os is supported
    let mut supported = false;
    SUPPORTED_OSES
        .into_iter()
        .for_each(|o| supported = o == pretty_name);

    supported
}

fn check_tdx_module() -> bool {
    let dmesg_output = Command::new("sudo")
        .arg("dmesg")
        .output()
        .expect("failed to run dmesg");

    let dmesg_output = String::from_utf8(dmesg_output.stdout)
        .expect("unable to convert utf8 bytes to owned String");

    dmesg_output.contains("virt/tdx: module initialized")
}

fn check_bios_tme_bypass() -> bool {
    let msr_value = Msr::new(0x982, 0).unwrap().read().unwrap();
    msr_value & (1 << 31) > 0
}

fn check_cpu_manufacturer_id() -> String {
    let res = unsafe { std::arch::x86_64::__cpuid(0x0000_0000) };
    let name: [u8; 12] = unsafe { std::mem::transmute([res.ebx, res.edx, res.ecx]) };
    String::from_utf8(name.to_vec()).unwrap()
}

fn check_kvm_supported() -> (TestState, String) {
    use std::os::fd::AsRawFd;

    match std::fs::File::open("/dev/kvm") {
        Ok(fd) => {
            let api_version = unsafe { libc::ioctl(fd.as_raw_fd(), 0xAE00, 0) };
            if api_version < 0 {
                (
                    TestState::Fail,
                    String::from("KVM device node (/dev/kvm) should be accessible"),
                )
            } else {
                (TestState::Ok, String::new())
            }
        }
        Err(_) => (
            TestState::Fail,
            String::from("Unable to read KVM device node file (/dev/kvm)"),
        ),
    }
}

fn check_kvm_module_supported(param: KvmParameter) -> (TestState, String, String) {
    let param_loc = match param {
        KvmParameter::Tdx => "/sys/module/kvm_intel/parameters/tdx",
        KvmParameter::Sgx => "/sys/module/kvm_intel/parameters/sgx",
    };

    let path = std::path::Path::new(param_loc);

    let (result, reason) = if path.exists() {
        match std::fs::read_to_string(param_loc) {
            Ok(result) => {
                if result.trim() == "1" || result.trim() == "Y" {
                    (TestState::Ok, String::new())
                } else {
                    (
                        TestState::Fail,
                        format!(
                            "Parameter file ({}) contains invalid value: {}",
                            param_loc, result
                        ),
                    )
                }
            }
            Err(e) => (
                TestState::Fail,
                format!("Unable to read parameter file: {}", e),
            ),
        }
    } else {
        (
            TestState::Fail,
            format!("Provided parameter does not exist: {}", param_loc),
        )
    };

    let action = format!(
        "Check /sys/module/kvm_intel/parameters/{} = Y (required)",
        param_loc[param_loc.rfind('/').unwrap() + 1..].to_owned()
    );

    (result, action, reason)
}

fn report_result(result: &mut TestResult) {
    let state = String::from(&result.state);

    match result.state {
        TestState::Ok => {
            println!("[ {} ] {}", state.green(), result.action);
        }
        TestState::Warning => {
            println!("[ {} ] {}", state.magenta(), result.action);
            if !result.reason.is_empty() {
                println!("\tReason: {}", result.reason.yellow());
            }
        }
        _ => {
            let mut color: &str = "red";
            if let TestOptionalState::Optional = result.optional_state {
                color = "yellow";
            }

            if let TestState::Tbd = result.state {
                color = "yellow";
            }

            if let TestOperationState::Manual = result.operation {
                color = "yellow";

                if let TestState::Fail = result.state {
                    color = "red";
                }

                result.reason = String::from("Unable to check in program. Please check manually.");
            }
            println!("[ {} ] {}", state.color(color), result.action);
            if !result.reason.is_empty() {
                let reason_str = format!("\tReason: {}", result.reason).color(color);
                println!("{}", reason_str);
            }
        }
    }
}

pub fn run_all_checks() -> Result<()> {
    println!("Required Features & Settings");
    println!("============================");
    let required_tests = get_required_tests();
    let required_tests_passed = run_test(&required_tests);

    println!();
    println!("Optional Features & Settings");
    println!("============================");
    let optional_tests = get_optional_tests();
    let _ = run_test(&optional_tests);

    if !required_tests_passed {
        Err(anyhow!("One or more required tests failed"))
    } else {
        Ok(())
    }
}

fn run_test(tests: &[Test]) -> bool {
    let mut passed = true;

    for t in tests {
        let mut res = (t.run)();
        report_result(&mut res);
        if let Some(f) = &t.post_run {
            (f)();
        }
        match res.state {
            TestState::Ok => {
                if !run_test(&t.sub_tests) {
                    passed = false;
                }
            }
            TestState::Fail => {
                passed = false;
                report_skip_result(&t.sub_tests);
            }
            TestState::Tbd => {}
            TestState::Skip => {}
            TestState::Warning => {}
        }
    }

    passed
}

fn report_skip_result(tests: &[Test]) {
    for t in tests {
        let res = TestResult {
            state: TestState::Skip,
            action: t.name.to_string(),
            ..Default::default()
        };
        let state = String::from(&res.state);
        println!("[ {} ] {}", state.yellow(), res.action);
        report_skip_result(&t.sub_tests);
    }
}

fn get_optional_tests() -> Vec<Test> {
    let bios_mem_map_test = Test {
        name: "Volatile Memory should be 1LM",
        run: Box::new(|| TestResult {
            action: String::from("Check BIOS: Volatile Memory should be 1LM"),
            state: TestState::Tbd,
            optional_state: TestOptionalState::Optional,
            operation: TestOperationState::Manual,
            ..Default::default()
        }),
        sub_tests: vec![],
        post_run: Some(Box::new(|| {
            println!("\tPlease check your BIOS settings:");
            println!("\t\tSocket Configuration -> Memory Configuration -> Memory Map");
            println!("\t\t\tVolatile Memory (or Volatile Memory Mode) should be 1LM");
            println!("\t\tA different BIOS might have a different path for this setting.");
            println!("\t\tPlease skip this setting if it doesn't exist in your BIOS menu.");
        })),
    };

    let bios_tme_bypass_test = Test {
        name: "TME Bypass is enabled",
        run: Box::new(|| {
            let state = if check_bios_tme_bypass() {
                TestState::Ok
            } else {
                TestState::Fail
            };

            TestResult {
                action: String::from("Check BIOS: TME Bypass = Enabled"),
                reason: String::from("The bit 31 of MSR 0x982 should be 1"),
                state,
                optional_state: TestOptionalState::Optional,
                ..Default::default()
            }
        }),
        sub_tests: vec![],
        post_run: Some(Box::new(|| {
            if !check_bios_tme_bypass() {
                println!("\tThe TME Bypass has not been enabled now.");
            }

            println!(
                "\tIt's better to enable TME Bypass for traditional non-confidential workloads."
            );
        })),
    };

    let bios_seam_loader_test = Test {
        name: "SEAM Loader is enabled",
        run: Box::new(|| TestResult {
            action: String::from("Check BIOS: SEAM Loader = Enabled"),
            state: TestState::Tbd,
            operation: TestOperationState::Manual,
            optional_state: TestOptionalState::Optional,
            ..Default::default()
        }),
        sub_tests: vec![],
        post_run: None,
    };

    vec![
        bios_mem_map_test,
        bios_tme_bypass_test,
        bios_seam_loader_test,
    ]
}

fn get_required_tests() -> Vec<Test> {
    //                       CPU Manufacturer ID
    //                                |
    //                                |
    //                          OS is supported
    //                                |
    //                                |
    //                          SGX is enabled
    //                                |
    //                                |
    //                          TDX is enabled
    //                                |
    //                                |
    //      +-------------------------+-----------------------+
    //      |             |           |          |            |
    //    TDX Mod.       TME       TME-MT     TDX Key      SGX Reg.
    //  Initialized    Enabled    Enabled    Split != 0    Server

    let tdx_enabled_test = Test {
        name: "Check TDX enabled",
        run: Box::new(|| {
            let msr_value = Msr::new(0x1401, 0).unwrap().read().unwrap();
            let state = if msr_value & (1 << 11) > 0 {
                TestState::Ok
            } else {
                TestState::Fail
            };
            TestResult {
                action: String::from("Check BIOS: TDX = Enabled"),
                reason: String::from("The bit 11 of MSR 0x1401 should be 1"),
                state,
                ..Default::default()
            }
        }),
        sub_tests: vec![
            Test {
                name: "Check TDX module initialized",
                run: Box::new(|| {
                    let module_initialized = check_tdx_module();
                    let state = if module_initialized {
                        TestState::Ok
                    } else {
                        TestState::Fail
                    };
                    TestResult {
                        action: String::from("Check TDX Module: The module is initialized"),
                        reason: String::from("TDX module is required"),
                        state,
                        ..Default::default()
                    }
                }),
                sub_tests: vec![],
                post_run: None,
            },
            Test {
                name: "Check TME enabled",
                run: Box::new(|| {
                    let msr_value = Msr::new(0x982, 0).unwrap().read().unwrap();
                    let state = if msr_value & (1 << 1) > 0 {
                        TestState::Ok
                    } else {
                        TestState::Fail
                    };
                    TestResult {
                        action: String::from("Check BIOS: TME = Enabled"),
                        reason: String::from("The bit 1 of MSR 0x982 should be 1."),
                        state,
                        ..Default::default()
                    }
                }),
                sub_tests: vec![],
                post_run: None,
            },
            Test {
                name: "Check TME-MT/TME-MK enabled",
                run: Box::new(|| {
                    let msr_value = Msr::new(0x982, 0).unwrap().read().unwrap();
                    let state = if msr_value & (1 << 1) > 0 {
                        TestState::Tbd
                    } else {
                        TestState::Fail
                    };
                    TestResult {
                        action: String::from("Check BIOS: TME-MT/TME-MK = Enabled"),
                        reason: String::from("The bit 1 of MSR 0x982 should be 1."),
                        state,
                        operation: TestOperationState::Manual,
                        ..Default::default()
                    }
                }),
                sub_tests: vec![],
                post_run: Some(Box::new(|| {
                    println!("\tPlease check your BIOS settings:");
                    println!(
                        "\t\tSocket Configuration -> Processor Configuration -> TME, TME-MT, TDX"
                    );
                    println!(
                        "\t\t\tTotal Memory Encryption Multi-Tenant (TME-MT) should be Enable"
                    );
                    println!("\t\tA different BIOS might have a different path for this setting.");
                })),
            },
            Test {
                name: "Check TDX Key Split != 0",
                run: Box::new(|| {
                    let msr_value = Msr::new(0x981, 0).unwrap().read().unwrap();
                    let state = if msr_value & (0x7fff << 36) != 0 {
                        TestState::Ok
                    } else {
                        TestState::Fail
                    };
                    TestResult {
                        action: String::from("Check BIOS: TDX Key Split != 0"),
                        reason: String::from("TDX Key Split should be non-zero"),
                        state,
                        ..Default::default()
                    }
                }),
                sub_tests: vec![],
                post_run: None,
            },
            Test {
                name: "Check SGX registration server",
                run: Box::new(|| TestResult {
                    action: String::from("Check BIOS: SGX registration server"),
                    reason: String::from(""),
                    state: TestState::Tbd,
                    operation: TestOperationState::Manual,
                    ..Default::default()
                }),
                sub_tests: vec![],
                post_run: Some(Box::new(|| {
                    let msr_value = Msr::new(0xce, 0).unwrap().read().unwrap();
                    if msr_value & (1 << 27) > 0 {
                        println!("\tSGX registration server is SBX");
                    } else {
                        println!("\tSGX registration server is LIV");
                    }
                })),
            },
        ],
        post_run: None,
    };

    let sgx_enabled_test = Test {
        name: "Check SGX enabled",
        run: Box::new(|| {
            let msr_value = Msr::new(0x3a, 0).unwrap().read().unwrap();
            let state = if msr_value & (1 << 18) > 0 {
                TestState::Ok
            } else {
                TestState::Fail
            };
            TestResult {
                action: String::from("Check BIOS: SGX = Enabled"),
                reason: String::from("The bit 18 of MSR 0x3a should be 1"),
                state,
                ..Default::default()
            }
        }),
        sub_tests: vec![tdx_enabled_test],
        post_run: None,
    };

    let os_distro_test = Test {
        name: "Check OS distro",
        run: Box::new(|| {
            let supported = check_os();
            let state = if supported {
                TestState::Ok
            } else {
                TestState::Fail
            };
            TestResult {
                action: String::from("Check OS: The distro and version are correct"),
                reason: String::from("Your OS distro is not supported yet."),
                state,
                ..Default::default()
            }
        }),
        sub_tests: vec![sgx_enabled_test],
        post_run: Some(Box::new(|| {
            let pretty_name = get_os_pretty_name();
            println!("\tYour current OS is: {}", pretty_name);
            println!("\tThe following OSs are supported:");
            for os in SUPPORTED_OSES {
                println!("\t\t{}", os);
            }
            println!("\tThere is no guarantee to other OS distros");
        })),
    };

    let cpu_manu_id_test = Test {
        name: "Check CPU Manufacturer ID",
        run: Box::new(|| {
            let manu_name = check_cpu_manufacturer_id();
            let state = if manu_name == "GenuineIntel" {
                TestState::Ok
            } else {
                TestState::Fail
            };
            TestResult {
                action: String::from("Check CPUID 0x0 Manufacturer ID = GenuineIntel"),
                reason: String::from("The CPUID Manufacturer ID should be GenuineIntel"),
                state,
                ..Default::default()
            }
        }),
        sub_tests: vec![os_distro_test],
        post_run: None,
    };

    //            KVM is enabled
    //                  |
    //                  |
    //      +----------------------+
    //      |                      |
    //     SGX                    TDX
    //  Mod Enabled           Mod Enabled

    let kvm_sgx_mod_test = Test {
        name: "Check KVM SGX parameter enabled",
        run: Box::new(|| {
            let (state, action, reason) = check_kvm_module_supported(KvmParameter::Sgx);
            TestResult {
                action,
                reason,
                state,
                ..Default::default()
            }
        }),
        sub_tests: vec![],
        post_run: None,
    };

    let kvm_tdx_mod_test = Test {
        name: "Check KVM TDX parameter enabled",
        run: Box::new(|| {
            let (state, action, reason) = check_kvm_module_supported(KvmParameter::Tdx);
            TestResult {
                action,
                reason,
                state,
                ..Default::default()
            }
        }),
        sub_tests: vec![],
        post_run: None,
    };

    let kvm_supported_test = Test {
        name: "Check KVM is supported",
        run: Box::new(|| {
            let (state, reason) = check_kvm_supported();
            TestResult {
                action: String::from("Check KVM is supported"),
                reason,
                state,
                ..Default::default()
            }
        }),
        sub_tests: vec![kvm_sgx_mod_test, kvm_tdx_mod_test],
        post_run: None,
    };

    vec![cpu_manu_id_test, kvm_supported_test]
}
