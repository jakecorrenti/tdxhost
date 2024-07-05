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

pub fn check_tdx_module() {}

pub fn check_bios_memory_map() {}

pub fn check_bios_tme_bypass() {}

pub fn check_bios_tme_mt() {}

pub fn check_bios_enabling_tdx() {}

pub fn check_bios_seam_loader() {}

pub fn check_bios_tdx_key_split() {}

pub fn check_bios_enabling_sgx() {}

pub fn check_bios_sgx_reg_server() {}

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

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_check_os() {
        check_os();
    }
}
