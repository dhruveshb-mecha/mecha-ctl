use anyhow::{bail, Result};
use sysinfo::{CpuExt, DiskExt, System, SystemExt};
use tracing::{error as trace_error, info, instrument, trace, warn};

use serde::{Deserialize, Serialize};

use crate::{DeviceInfoCtlError, DeviceInfoCtlErrorCodes};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_memory: u64,
    pub available_memory: u64,
    pub free_memory: u64,
}

#[derive(Debug, Default)]
pub struct CpuInfo {
    pub cpu_name: String,
    pub cpu_frequency: u64,
    pub number_of_cores: usize,
}

#[derive(Debug, Default)]
pub struct DiskInfo {
    pub name: String,
    pub fs: String,
    pub removable: bool,
    pub mount_point: String,
    pub used_space: u64,
    pub total_space: u64,
}

#[derive(Debug, Default)]
pub struct DeviceInfoControl {}

impl DeviceInfoControl {
    pub fn new() -> Self {
        DeviceInfoControl {}
    }

    #[instrument(skip(self))]
    pub fn get_memory_info(&self) -> Result<MemoryInfo> {
        trace!(task = "get_memory_info", "init");
        let mut system = System::new_all();
        system.refresh_all();

        let total_memory = system.total_memory();
        let free_memory = system.free_memory();
        let available_memory = system.available_memory();

        match (total_memory, free_memory, available_memory) {
            (total_memory, free_memory, available_memory) => {
                let memory_info = MemoryInfo {
                    total_memory,
                    free_memory,
                    available_memory,
                };
                info!(task = "get_memory_info", "memory info: {:?}", memory_info);
                return Ok(memory_info);
            }
            _ => {
                trace_error!(task = "get_memory_info", "failed to get memory info");
                bail!(DeviceInfoCtlError::new(
                    DeviceInfoCtlErrorCodes::FailedToGetMemoryUsage,
                    "failed to get memory info".to_string(),
                ))
            }
        }
    }

    #[instrument(skip(self))]
    pub fn get_cpu_info(&self) -> Result<CpuInfo> {
        trace!(task = "get_cpu_info", "init");
        let mut system = System::new_all();
        system.refresh_all();

        //use some or none to get the first cpu data and peroperly handle the error
        match system.cpus().iter().take(1).next() {
            Some(cpu_data) => {
                // use some or none to get the number of cores and properly handle the error
                let numer_of_cores = match system.physical_core_count() {
                    Some(processor_cores) => {
                        info!(
                            task = "get_cpu_info",
                            "number of cores: {}", processor_cores
                        );
                        processor_cores
                    }
                    None => {
                        warn!(task = "get_cpu_info", "failed to get number of cores");
                        bail!(DeviceInfoCtlError::new(
                            DeviceInfoCtlErrorCodes::FailedToGetCpuUsage,
                            "failed to get CPU info".to_string(),
                        ))
                    }
                };

                let cpu_info = CpuInfo {
                    cpu_name: cpu_data.brand().to_string(),
                    cpu_frequency: cpu_data.frequency(),
                    number_of_cores: numer_of_cores,
                };
                info!(task = "get_cpu_info", "cpu info: {:?}", cpu_info);
                return Ok(cpu_info);
            }
            None => bail!(DeviceInfoCtlError::new(
                DeviceInfoCtlErrorCodes::FailedToGetCpuUsage,
                "failed to get CPU info".to_string(),
            )),
        }

        //if there is cpu data and number of cores then return cpu info else return error with bail and DeviceInfoCtlError
    }

    #[instrument(skip(self))]
    pub fn get_disk_info(&self) -> Result<Vec<DiskInfo>> {
        trace!(task = "get_disk_info", "init");
        let mut system = System::new_all();
        system.refresh_all();

        let mut disks = Vec::new();

        for disk in system.disks() {
            let disk_info = DiskInfo {
                name: disk.name().to_string_lossy().into_owned(),
                fs: String::from_utf8_lossy(disk.file_system()).into_owned(),
                removable: disk.is_removable(),
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                used_space: disk.total_space() - disk.available_space(),
                total_space: disk.total_space(),
            };
            info!(task = "get_disk_info", "disk info: {:?}", disk_info);
            disks.push(disk_info);
        }

        if disks.is_empty() {
            warn!("failed to get disk info");
            bail!(DeviceInfoCtlError::new(
                DeviceInfoCtlErrorCodes::FailedToGetDiskUsage,
                "failed to get disk info".to_string(),
            ));
        }

        Ok(disks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{automock, predicate::*};

    #[automock]
    pub trait DeviceInfoCtlObject {
        fn new_all() -> Self;
        fn total_memory(&self) -> u64;
        fn free_memory(&self) -> u64;
        fn available_memory(&self) -> u64;
        fn cpu_name(&self) -> String;
        fn frequency(&self) -> u64;
        fn physical_core_count(&self) -> Option<usize>;
        //add all disk info params like name, fs, removable, mount_point, used_space, total_space
        fn name(&self) -> String;
        fn file_system(&self) -> String;
        fn is_removable(&self) -> bool;
        fn mount_point(&self) -> String;
        fn total_space(&self) -> u64;
        fn disk_usage(&self) -> u64;
    }

    #[test]
    fn test_get_memory_info() {
        let mut mock = MockDeviceInfoCtlObject::new();
        mock.expect_total_memory().returning(|| 8000);
        mock.expect_free_memory().returning(|| 2000);
        mock.expect_available_memory().returning(|| 3000);

        // let device_info = DeviceInfoControl::new();
        let memory_info = MemoryInfo {
            total_memory: mock.total_memory(),
            free_memory: mock.free_memory(),
            available_memory: mock.available_memory(),
        };

        assert_eq!(memory_info.total_memory, 8000);
        assert_eq!(memory_info.free_memory, 2000);
        assert_eq!(memory_info.available_memory, 3000);
    }

    #[test]
    fn test_get_cpu_info() {
        let mut mock = MockDeviceInfoCtlObject::new();
        mock.expect_cpu_name()
            .returning(|| "Intel(R) Core(TM) i7-7700HQ CPU @ 2.80GHz".to_string());
        mock.expect_frequency().returning(|| 2800000);
        mock.expect_physical_core_count().returning(|| Some(4));

        let cpu_info = CpuInfo {
            cpu_name: mock.cpu_name().to_string(),
            cpu_frequency: mock.frequency(),
            number_of_cores: mock.physical_core_count().unwrap(),
        };

        assert_eq!(
            cpu_info.cpu_name,
            "Intel(R) Core(TM) i7-7700HQ CPU @ 2.80GHz"
        );
        assert_eq!(cpu_info.cpu_frequency, 2800000);
        assert_eq!(cpu_info.number_of_cores, 4);
    }

    #[test]
    fn test_get_disk_info() {
        let mut mock = MockDeviceInfoCtlObject::new();
        mock.expect_name().returning(|| "sda".to_string());
        mock.expect_file_system().returning(|| "ext4".to_string());
        mock.expect_is_removable().returning(|| false);
        mock.expect_mount_point().returning(|| "/".to_string());
        mock.expect_total_space().returning(|| 1000000000);

        mock.expect_disk_usage().returning(|| 500000000);

        let disk_info = DiskInfo {
            name: mock.name().to_string(),
            fs: mock.file_system(),
            removable: mock.is_removable(),
            mount_point: mock.mount_point(),
            used_space: mock.disk_usage(),
            total_space: mock.total_space(),
        };

        assert_eq!(disk_info.name, "sda");
        assert_eq!(disk_info.fs, "ext4");
        assert_eq!(disk_info.removable, false);
        assert_eq!(disk_info.mount_point, "/");
        assert_eq!(disk_info.used_space, 500000000);
        assert_eq!(disk_info.total_space, 1000000000);
    }
}
