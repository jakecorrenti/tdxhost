use msru::{Accessor, Msr};
use std::process::Command;

enum TestResult {
    Ok,
    Failed,
    #[allow(dead_code)]
    Warning,
    Tbd,
}

impl From<&TestResult> for String {
    fn from(res: &TestResult) -> Self {
        match res {
            TestResult::Ok => "OK".to_string(),
            TestResult::Failed => "FAILED".to_string(),
            TestResult::Warning => "WARNING".to_string(),
            TestResult::Tbd => "TBD".to_string(),
        }
    }
}

enum TestOptionalState {
    Required,
    Optional,
}

#[derive(Default)]
enum TestOperation {
    Manual,
    #[default]
    Program,
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

pub fn check_os() {
    // get os name
    let pretty_name = get_os_pretty_name();

    // check if the os is supported
    let mut supported = false;
    SUPPORTED_OSES
        .into_iter()
        .for_each(|o| supported = o == pretty_name);

    // report the result
    report_results(
        if supported {
            TestResult::Ok
        } else {
            TestResult::Failed
        },
        "Check OS: The distro and version are correct (required)",
        "Your OS distro is not supported yet.",
        TestOptionalState::Required,
        None,
    );

    // print os information
    println!("\tYour current OS is: {}", pretty_name);
    println!("\tThe following OSs are supported:");
    for os in SUPPORTED_OSES {
        println!("\t\t{}", os);
    }
    println!("\tThere is no guarantee to other OS distros");
}

pub fn check_tdx_module() {
    let dmesg_output = Command::new("sudo")
        .arg("dmesg")
        .output()
        .expect("failed to run dmesg");

    let dmesg_output = String::from_utf8(dmesg_output.stdout)
        .expect("unable to convert utf8 bytes to owned String");

    if dmesg_output.contains("virt/tdx: module initialized") {
        report_results(
            TestResult::Ok,
            "Check TDX Module: The module is initialized (required)",
            "",
            TestOptionalState::Required,
            None,
        );
    } else {
        report_results(
            TestResult::Failed,
            "Check TDX Module: The module is initialized (required)",
            "TDX Module is required",
            TestOptionalState::Required,
            None,
        );
    }
}

pub fn check_bios_memory_map() {
    report_results(
        TestResult::Tbd,
        "Check BIOS: Volatile Memory should be 1LM (optional & manually)",
        "",
        TestOptionalState::Optional,
        Some(TestOperation::Manual),
    );

    println!("\tPlease check your BIOS settings:");
    println!("\t\tSocket Configuration -> Memory Configuration -> Memory Map");
    println!("\t\t\tVolatile Memory (or Volatile Memory Mode) should be 1LM");
    println!("\t\tA different BIOS might have a different path for this setting.");
    println!("\t\tPlease skip this setting if it doesn't exist in your BIOS menu.");
}

pub fn check_bios_enabling_mktme() {
    let msr_value = Msr::new(0x982, 0).unwrap().read().unwrap();

    report_results(
        if msr_value & (1 << 1) > 0 {
            TestResult::Ok
        } else {
            TestResult::Failed
        },
        "Check BIOS: TME = Enabled (required)",
        "The bit 1 of MSR 0x982 should be 1",
        TestOptionalState::Required,
        None,
    );
}

pub fn check_bios_tme_bypass() {
    let msr_value = Msr::new(0x982, 0).unwrap().read().unwrap();

    let tme_bypass_enabled = msr_value & (1 << 31) > 0;
    report_results(
        if tme_bypass_enabled {
            TestResult::Ok
        } else {
            TestResult::Failed
        },
        "Check BIOS: TME Bypass = Enabled (optional)",
        "The bit 31 of MSR 0x982 should be 1",
        TestOptionalState::Optional,
        None,
    );

    if !tme_bypass_enabled {
        println!("\tThe TME Bypass has not been enabled now.");
    }

    println!("\tIt's better to enable TME Bypass for traditional non-confidential workloads.");
}

pub fn check_bios_tme_mt() {
    let msr_value = Msr::new(0x982, 0).unwrap().read().unwrap();

    report_results(
        if msr_value & (1 << 1) > 0 {
            TestResult::Ok
        } else {
            TestResult::Failed
        },
        "Check BIOS: TME-MT/TME-MK (required & manually)",
        "The bit 1 of MSR 0x982 should be 1",
        TestOptionalState::Required,
        None,
    );

    println!("\tPlease check your BIOS settings:");
    println!("\t\tSocket Configuration -> Processor Configuration -> TME, TME-MT, TDX");
    println!("\t\t\tTotal Memory Encryption Multi-Tenant (TME-MT) should be Enable");
    println!("\t\tA different BIOS might have a different path for this setting.");
}

pub fn check_bios_enabling_tdx() {
    let msr_value = Msr::new(0x1401, 0).unwrap().read().unwrap();

    report_results(
        if msr_value & (1 << 11) > 0 {
            TestResult::Ok
        } else {
            TestResult::Failed
        },
        "Check BIOS: TDX = Enabled (required)",
        "The bit 11 of MSR 0x1401 should be 1",
        TestOptionalState::Required,
        None,
    );
}

pub fn check_bios_seam_loader() {
    report_results(
        TestResult::Tbd,
        "Check BIOS: SEAM Loader = Enabled (optional)",
        "",
        TestOptionalState::Optional,
        Some(TestOperation::Manual),
    );
}

pub fn check_bios_tdx_key_split() {
    let msr_value = Msr::new(0x981, 0).unwrap().read().unwrap();

    report_results(
        // check bits 50:36
        if msr_value & (0x7fff << 36) != 0 {
            TestResult::Ok
        } else {
            TestResult::Failed
        },
        "Check BIOS: TDX Key Split != 0 (required)",
        "TDX Key Split should be non-zero",
        TestOptionalState::Required,
        None,
    );
}

pub fn check_bios_enabling_sgx() {
    let msr_value = Msr::new(0x3a, 0).unwrap().read().unwrap();

    report_results(
        if msr_value & (1 << 18) > 0 {
            TestResult::Ok
        } else {
            TestResult::Failed
        },
        "Check BIOS: SGX = Enabled (required)",
        "The bit 18 of MSR 0x3a should be 1",
        TestOptionalState::Required,
        None,
    );
}

pub fn check_bios_sgx_reg_server() {
    let msr_value = Msr::new(0xce, 0).unwrap().read().unwrap();

    report_results(
        TestResult::Tbd,
        "Check BIOS: SGX registration server (required & manually)",
        "",
        TestOptionalState::Required,
        Some(TestOperation::Manual),
    );

    if msr_value & (1 << 27) > 0 {
        println!("\tSGX registration server is SBX");
    } else {
        println!("\tSGX registration server is LIV");
    }
}

pub fn check_cpu_manufacturer_id() {
    let res = unsafe { std::arch::x86_64::__cpuid(0x0000_0000) };
    let name: [u8; 12] = unsafe { std::mem::transmute([res.ebx, res.edx, res.ecx]) };
    let name = String::from_utf8(name.to_vec()).unwrap();

    report_results(
        if name == "GenuineIntel" {
            TestResult::Ok
        } else {
            TestResult::Failed
        },
        "Check CPUID 0x0 Manufacturer ID = GenuineIntel (required)",
        "The CPUID Manufacturer ID should be GenuineIntel",
        TestOptionalState::Required,
        None,
    );
}

pub fn check_kvm_supported() {
    use std::os::fd::AsRawFd;

    let (result, reason) = match std::fs::File::open("/dev/kvm") {
        Ok(fd) => {
            let api_version = unsafe { libc::ioctl(fd.as_raw_fd(), 0xAE00, 0) };
            if api_version < 0 {
                (
                    TestResult::Failed,
                    "KVM device node (/dev/kvm) should be accessible",
                )
            } else {
                (TestResult::Ok, "")
            }
        }
        Err(_) => (
            TestResult::Failed,
            "Unable to read KVM device node file (/dev/kvm)",
        ),
    };

    report_results(
        result,
        "Check KVM is supported (required)",
        reason,
        TestOptionalState::Required,
        None,
    );
}

fn report_results(
    result: TestResult,
    action: &str,
    reason: &str,
    optional: TestOptionalState,
    operation: Option<TestOperation>,
) {
    use colored::Colorize;
    let mut reason = reason;
    let res = String::from(&result);

    match result {
        TestResult::Ok => {
            println!("[ {} ] {}", res.green(), action);
        }
        TestResult::Warning => {
            println!("[ {} ] {}", res.magenta(), action);
            if !reason.is_empty() {
                println!("\tReason: {}", reason.yellow());
            }
        }
        _ => {
            let mut color: &str = "red";
            if let TestOptionalState::Optional = optional {
                color = "yellow";
            }

            if operation.is_some() {
                if let TestOperation::Manual = operation.unwrap() {
                    color = "yellow";
                    reason = "Unable to check in program. Please check manually.";
                }
            }
            println!("[ {} ] {}", res.color(color), action);
            if !reason.is_empty() {
                let reason_str = format!("\tReason: {}", reason).color(color);
                println!("{}", reason_str);
            }
        }
    }
}

pub fn run_all_checks() {
    println!("Required Features & Settings");
    println!("============================");
    check_os();
    check_tdx_module();
    check_bios_enabling_mktme();
    check_bios_tme_mt();
    check_bios_enabling_tdx();
    check_bios_tdx_key_split();
    check_bios_enabling_sgx();
    check_bios_sgx_reg_server();
    check_cpu_manufacturer_id();
    check_kvm_supported();

    println!();
    println!("Optional Features & Settings");
    println!("============================");
    check_bios_memory_map();
    check_bios_tme_bypass();
    check_bios_seam_loader();
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_check_os() {
        check_os();
    }

    #[test]
    fn test_check_bios_memory_map() {
        check_bios_memory_map();
    }

    #[test]
    fn test_check_bios_enabling_mktme() {
        check_bios_enabling_mktme();
    }

    #[test]
    fn test_check_bios_tme_bypass() {
        check_bios_tme_bypass();
    }

    #[test]
    fn test_check_bios_tme_mt() {
        check_bios_tme_mt();
    }

    #[test]
    fn test_check_bios_enabling_tdx() {
        check_bios_enabling_tdx();
    }

    #[test]
    fn test_check_bios_seam_loader() {
        check_bios_seam_loader();
    }

    #[test]
    fn test_check_bios_tdx_key_split() {
        check_bios_tdx_key_split();
    }

    #[test]
    fn test_check_bios_enabling_sgx() {
        check_bios_enabling_sgx();
    }

    #[test]
    fn test_check_bios_sgx_reg_server() {
        check_bios_sgx_reg_server();
    }

    #[test]
    fn test_check_tdx_module() {
        check_tdx_module();
    }

    #[test]
    fn test_check_cpu_manufacturer_id() {
        check_cpu_manufacturer_id();
    }

    #[test]
    fn test_check_kvm_supported() {
        check_kvm_supported();
    }
}
