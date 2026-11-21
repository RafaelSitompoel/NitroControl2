use crate::error::{NitroError, Result};
use std::process::Command;
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub struct AcerWmiInterface;

#[derive(Clone, Debug)]
pub enum WmiProbeResult {
    Accessible,
    PermissionDenied(String),
    NotAvailable(String),
}

static PROBE_CACHE: RwLock<Option<(Instant, WmiProbeResult)>> = RwLock::new(None);
const PROBE_CACHE_DURATION: Duration = Duration::from_secs(30);

impl AcerWmiInterface {
    pub fn probe_acer_gaming_wmi() -> WmiProbeResult {
        if let Ok(guard) = PROBE_CACHE.read() {
            if let Some((instant, ref res)) = *guard {
                if instant.elapsed() < PROBE_CACHE_DURATION {
                    return res.clone();
                }
            }
        }

        // Try AcerGamingFunction first (older models), then AcerGaming (Nitro V15/newer)
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                r#"
$ErrorActionPreference = 'Stop'
$found = $false
# Try primary class first
try {
    $c = Get-CimClass -Namespace root\wmi -ClassName AcerGamingFunction -ErrorAction Stop
    if ($null -ne $c) {
        try {
            [void](Get-CimInstance -Namespace root\wmi -ClassName AcerGamingFunction -ErrorAction Stop)
            Write-Output 'ACCESSIBLE:AcerGamingFunction'
            $found = $true
        } catch {
            if ($_.Exception.Message -match 'Access denied') {
                Write-Output 'PERMISSION_DENIED'
                $found = $true
            }
        }
    }
} catch {}

if (-not $found) {
    # Try fallback class (some Nitro V15 / newer models)
    try {
        $c2 = Get-CimClass -Namespace root\wmi -ClassName AcerGaming -ErrorAction Stop
        if ($null -ne $c2) {
            try {
                [void](Get-CimInstance -Namespace root\wmi -ClassName AcerGaming -ErrorAction Stop)
                Write-Output 'ACCESSIBLE:AcerGaming'
                $found = $true
            } catch {
                if ($_.Exception.Message -match 'Access denied') {
                    Write-Output 'PERMISSION_DENIED'
                    $found = $true
                }
            }
        }
    } catch {}
}

if (-not $found) { Write-Output 'NOT_FOUND' }
"#,
            ])
            .output();

        let result = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if stdout.contains("ACCESSIBLE") {
                    WmiProbeResult::Accessible
                } else if stdout.contains("PERMISSION_DENIED") {
                    WmiProbeResult::PermissionDenied(
                        "Administrator privileges are required to access Acer Gaming WMI hardware registers."
                            .to_string(),
                    )
                } else {
                    WmiProbeResult::NotAvailable(
                        "AcerGamingFunction / AcerGaming WMI class is not available on this system."
                            .to_string(),
                    )
                }
            }
            Err(e) => WmiProbeResult::NotAvailable(format!("Failed to execute WMI probe: {}", e)),
        };

        if let Ok(mut guard) = PROBE_CACHE.write() {
            *guard = Some((Instant::now(), result.clone()));
        }

        result
    }

    /// Detect which WMI class is available (AcerGamingFunction or AcerGaming)
    fn get_wmi_classname() -> &'static str {
        // Try to detect once; default to AcerGamingFunction
        // The probe result above doesn't store which class was found, so we re-check quickly
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "try { [void](Get-CimClass -Namespace root\\wmi -ClassName AcerGamingFunction -ErrorAction Stop); 'AcerGamingFunction' } catch { try { [void](Get-CimClass -Namespace root\\wmi -ClassName AcerGaming -ErrorAction Stop); 'AcerGaming' } catch { 'AcerGamingFunction' } }",
            ])
            .output();

        match output {
            Ok(out) => {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if s == "AcerGaming" {
                    "AcerGaming"
                } else {
                    "AcerGamingFunction"
                }
            }
            Err(_) => "AcerGamingFunction",
        }
    }

    /// Battery limiter — Acer NitroSense uses SetGamingMiscSetting with:
    ///   byte[0] = 5  → battery health limiter function
    ///   byte[1] = 0  → disable limit (charge to 100%)
    ///   byte[1] = 1  → enable 80% charge limit (Acer BattCare)
    pub fn set_battery_limiter(enabled: bool) -> Result<()> {
        match Self::probe_acer_gaming_wmi() {
            WmiProbeResult::PermissionDenied(reason) => Err(NitroError::PermissionDenied(reason)),
            WmiProbeResult::NotAvailable(reason) => Err(NitroError::HardwareNotSupported(reason)),
            WmiProbeResult::Accessible => {
                let class = Self::get_wmi_classname();
                let limit_byte: u8 = if enabled { 1 } else { 0 };
                let script = format!(
                    "try {{ \
                     $inst = Get-CimInstance -Namespace root\\wmi -ClassName {class} -ErrorAction Stop; \
                     $inp = [byte[]](5, {limit_byte}, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0); \
                     $res = Invoke-CimMethod -InputObject $inst -MethodName SetGamingMiscSetting -Arguments @{{ gmInput = $inp }} -ErrorAction Stop; \
                     'SUCCESS' }} catch {{ 'FAIL:' + $_.Exception.Message }}"
                );

                let output = Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                    .output()
                    .map_err(|e| NitroError::OperationFailed(e.to_string()))?;

                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout.contains("SUCCESS") {
                    Ok(())
                } else {
                    Err(NitroError::OperationFailed(format!(
                        "Battery limiter WMI command failed: {}",
                        stdout
                    )))
                }
            }
        }
    }

    /// Read current battery limiter state
    pub fn get_battery_limiter() -> Result<bool> {
        match Self::probe_acer_gaming_wmi() {
            WmiProbeResult::PermissionDenied(reason) => Err(NitroError::PermissionDenied(reason)),
            WmiProbeResult::NotAvailable(reason) => Err(NitroError::HardwareNotSupported(reason)),
            WmiProbeResult::Accessible => {
                let class = Self::get_wmi_classname();
                let script = format!(
                    "try {{ \
                    $inst = Get-CimInstance -Namespace root\\wmi -ClassName {class} -ErrorAction Stop; \
                    $inp = [byte[]](5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0); \
                    $res = Invoke-CimMethod -InputObject $inst -MethodName GetGamingMiscSetting -Arguments @{{ gmInput = $inp }} -ErrorAction Stop; \
                    'VALUE:' + [int]$res.gmOutput[1] }} catch {{ 'FAIL:' + $_.Exception.Message }}"
                );

                let output = Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                    .output()
                    .map_err(|e| NitroError::OperationFailed(e.to_string()))?;

                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout.starts_with("VALUE:") {
                    let val: u8 = stdout.trim_start_matches("VALUE:").trim().parse().unwrap_or(0);
                    Ok(val == 1)
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// Set gaming fan mode — supports Auto, Max, Custom
    /// For Nitro V15 (AN515-58 etc.) uses SetGamingFanBehavior with correct mode codes.
    /// Falls back to SetGamingFanSpeed for models that need explicit speed setting.
    pub fn set_gaming_fan_mode(mode: &str, speed_pct: Option<u32>) -> Result<()> {
        match Self::probe_acer_gaming_wmi() {
            WmiProbeResult::PermissionDenied(reason) => Err(NitroError::PermissionDenied(reason)),
            WmiProbeResult::NotAvailable(reason) => Err(NitroError::HardwareNotSupported(reason)),
            WmiProbeResult::Accessible => {
                let class = Self::get_wmi_classname();

                // Mode codes for SetGamingFanBehavior:
                // 0 = Auto (firmware manages fan speed)
                // 1 = Max  (full speed)
                // 2 = Custom / Manual (requires speed value)
                let mode_code: u32 = match mode {
                    "Max" | "max" => 1,
                    "Custom" | "custom" => 2,
                    _ => 0, // Auto
                };

                // Primary: try SetGamingFanBehavior
                let script_behavior = format!(
                    "try {{ \
                     $inst = Get-CimInstance -Namespace root\\wmi -ClassName {class} -ErrorAction Stop; \
                     $res = Invoke-CimMethod -InputObject $inst -MethodName SetGamingFanBehavior \
                         -Arguments @{{ gmInput = {mode_code} }} -ErrorAction Stop; \
                     'SUCCESS' }} catch {{ 'FAIL:' + $_.Exception.Message }}"
                );

                let out1 = Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script_behavior])
                    .output()
                    .map_err(|e| NitroError::OperationFailed(e.to_string()))?;

                let s1 = String::from_utf8_lossy(&out1.stdout).trim().to_string();
                if s1.contains("SUCCESS") {
                    // For Custom mode, also set the speed percentage
                    if mode_code == 2 {
                        if let Some(pct) = speed_pct {
                            let _ = Self::set_fan_speed_percent(class, pct);
                        }
                    }
                    return Ok(());
                }

                // Fallback: try SetGamingFanSpeed directly (some Nitro V15 firmware variants)
                // CPU fan = gmInput[0], GPU fan = gmInput[1], values 0-255 (mapped from 0-100%)
                let speed_val = match mode {
                    "Max" | "max" => 255u8,
                    "Custom" | "custom" => {
                        let pct = speed_pct.unwrap_or(60).min(100);
                        ((pct as f32 / 100.0) * 255.0) as u8
                    }
                    _ => 0u8, // Auto = 0 means firmware control
                };

                let script_speed = format!(
                    "try {{ \
                     $inst = Get-CimInstance -Namespace root\\wmi -ClassName {class} -ErrorAction Stop; \
                     $inp = [byte[]]({speed_val}, {speed_val}, 0, 0, 0, 0, 0, 0); \
                     $res = Invoke-CimMethod -InputObject $inst -MethodName SetGamingFanSpeed \
                         -Arguments @{{ gmInput = $inp }} -ErrorAction Stop; \
                     'SUCCESS' }} catch {{ 'FAIL2:' + $_.Exception.Message }}"
                );

                let out2 = Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script_speed])
                    .output()
                    .map_err(|e| NitroError::OperationFailed(e.to_string()))?;

                let s2 = String::from_utf8_lossy(&out2.stdout).trim().to_string();
                if s2.contains("SUCCESS") {
                    return Ok(());
                }

                Err(NitroError::OperationFailed(format!(
                    "Fan mode WMI commands failed.\nBehavior: {}\nSpeed: {}",
                    s1, s2
                )))
            }
        }
    }

    fn set_fan_speed_percent(class: &str, pct: u32) -> Result<()> {
        let pct_clamped = pct.min(100);
        // Some models use 0-100 scale, others 0-255; try 0-100 first
        let script = format!(
            "try {{ \
             $inst = Get-CimInstance -Namespace root\\wmi -ClassName {class} -ErrorAction Stop; \
             $inp = [byte[]]({pct_clamped}, {pct_clamped}, 0, 0, 0, 0, 0, 0); \
             $res = Invoke-CimMethod -InputObject $inst -MethodName SetGamingFanSpeed \
                 -Arguments @{{ gmInput = $inp }} -ErrorAction Stop; \
             'SUCCESS' }} catch {{ 'FAIL:' + $_.Exception.Message }}"
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| NitroError::OperationFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.contains("SUCCESS") {
            Ok(())
        } else {
            Err(NitroError::OperationFailed(stdout))
        }
    }
}
