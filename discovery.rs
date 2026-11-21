use crate::error::{NitroError, Result};
use crate::types::system::SystemInfo;
use std::process::Command;
use std::sync::OnceLock;
use sysinfo::System;

pub struct HardwareDiscovery;

static CACHED_SYS_INFO: OnceLock<SystemInfo> = OnceLock::new();

impl HardwareDiscovery {
    pub fn get_system_info() -> Result<SystemInfo> {
        if let Some(info) = CACHED_SYS_INFO.get() {
            return Ok(info.clone());
        }

        let mut sys = System::new_all();
        sys.refresh_all();

        let os_name = System::name().unwrap_or_else(|| "Windows".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
        let hostname = System::host_name().unwrap_or_else(|| "Localhost".to_string());
        let uptime_secs = System::uptime();

        let (model, manufacturer) = Self::query_model_and_mfg()
            .unwrap_or_else(|_| ("Standard PC".to_string(), "Unknown".to_string()));
        let bios_version = Self::query_bios_version()
            .unwrap_or_else(|_| "Unknown BIOS".to_string());

        let info = SystemInfo {
            model,
            manufacturer,
            os_name,
            os_version,
            bios_version,
            hostname,
            uptime_secs,
        };

        let _ = CACHED_SYS_INFO.set(info.clone());
        Ok(info)
    }

    pub fn is_acer_nitro(model: &str, mfg: &str) -> bool {
        let mfg_lower = mfg.to_lowercase();
        let model_lower = model.to_lowercase();

        let is_acer = mfg_lower.contains("acer") || model_lower.contains("acer");

        // Nitro V15 models: AN515-58, AN515-57, AN515-56, AN515-55, AN515-51
        // Nitro V: ANV15-51, ANV15-52
        // Nitro 5: various AN5xx
        // Nitro 16: AN16-xx
        // Nitro 17: AN17-xx
        let is_nitro = model_lower.contains("nitro")
            || model_lower.contains("an5")
            || model_lower.contains("anv")
            || model_lower.contains("an16")
            || model_lower.contains("an17")
            // Explicit Nitro V15 model numbers (Acer AN515-xx series)
            || model_lower.contains("an515")
            || model_lower.contains("an516")
            || model_lower.contains("an517")
            || model_lower.contains("an518")
            || model_lower.contains("an519")
            // Nitro V (new series)
            || model_lower.contains("anv15")
            || model_lower.contains("anv16");

        is_acer && is_nitro
    }

    pub fn detect_nitro_generation(model: &str) -> Option<String> {
        let model_lower = model.to_lowercase();

        // Nitro V (new branding, 2023+)
        if model_lower.contains("anv15") || model_lower.contains("nitro v 15") {
            return Some("Nitro V 15".to_string());
        }
        if model_lower.contains("anv16") || model_lower.contains("nitro v 16") {
            return Some("Nitro V 16".to_string());
        }
        if model_lower.contains("anv") || model_lower.contains("nitro v") {
            return Some("Nitro V".to_string());
        }

        // Nitro 5 (AN515-xx series = 15.6" Nitro 5 / V15)
        if model_lower.contains("an515-58") || model_lower.contains("an515-57") {
            return Some("Nitro V15 (12th/13th Gen)".to_string());
        }
        if model_lower.contains("an515-56") || model_lower.contains("an515-55") {
            return Some("Nitro 5 (11th Gen)".to_string());
        }
        if model_lower.contains("an515") {
            return Some("Nitro 5 / V15".to_string());
        }

        // Nitro 16/17
        if model_lower.contains("an16") || model_lower.contains("nitro 16") {
            return Some("Nitro 16".to_string());
        }
        if model_lower.contains("an17") || model_lower.contains("nitro 17") {
            return Some("Nitro 17".to_string());
        }

        // Nitro 5 generic
        if model_lower.contains("an5") || model_lower.contains("nitro 5") {
            return Some("Nitro 5".to_string());
        }

        // Generic Nitro
        if model_lower.contains("nitro") {
            return Some("Nitro".to_string());
        }

        None
    }

    fn query_model_and_mfg() -> Result<(String, String)> {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "try { $cs = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop; \"$($cs.Model)|$($cs.Manufacturer)\" } catch { 'Unknown|Unknown' }",
            ])
            .output()
            .map_err(|e| NitroError::Internal(format!("Failed to execute PowerShell query: {}", e)))?;

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = text.split('|').collect();

        if parts.len() == 2 {
            let model = parts[0].trim().to_string();
            let mfg = parts[1].trim().to_string();
            Ok((
                if model.is_empty() { "Unknown Model".into() } else { model },
                if mfg.is_empty() { "Unknown Manufacturer".into() } else { mfg },
            ))
        } else {
            Ok(("Unknown Model".into(), "Unknown Manufacturer".into()))
        }
    }

    fn query_bios_version() -> Result<String> {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "try { (Get-CimInstance Win32_Bios -ErrorAction Stop).SMBIOSBIOSVersion } catch { 'Unknown' }",
            ])
            .output()
            .map_err(|e| NitroError::Internal(e.to_string()))?;

        let bios = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(if bios.is_empty() { "Unknown BIOS".into() } else { bios })
    }
}
