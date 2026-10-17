//! Refuses to start a real-window run on a Mac that cannot show one.
//!
//! With the lid closed the game's window is never presented, so a run does
//! not fail where the cause is: it waits about 100 seconds and then reports
//! "presentation N was not acknowledged". This check names the cause before
//! the game is launched.
//!
//! Both facts come from one read-only command, `ioreg -r -k AppleClamshellState
//! -d 1`, which prints the power root domain:
//!
//! * `"AppleClamshellState" = Yes` means the lid is closed;
//! * `"System Capabilities"` is the power-management capability mask. Bit 1
//!   (`0x2`) is graphics, which is on only while a display is awake. A dark
//!   wake or a sleeping display reports a mask without it (for example 9),
//!   an awake display reports one with it (for example 15).
//!
//! A Mac with no lid (a Mac mini) prints no `AppleClamshellState` key; it has
//! no closed lid to refuse on and the display check is skipped, because the
//! mask is not printed for it either.

use std::process::Command;

use crate::display_awake::{hold_display_awake, DisplayAwake};

/// The system tool that prints the power root domain.
const IOREG: &str = "/usr/sbin/ioreg";

/// `kIOPMSystemCapabilityGraphics`: set while the display is powered.
const CAPABILITY_GRAPHICS: u64 = 0x2;

/// What the power root domain says about the lid and the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostPower {
    pub lid_closed: bool,
    /// `None` when the capability mask was not printed.
    pub graphics_on: Option<bool>,
}

impl HostPower {
    /// Parse the output of `ioreg -r -k AppleClamshellState -d 1`.
    pub fn parse(output: &str) -> Result<Self, String> {
        let lid_closed = match property(output, "AppleClamshellState") {
            Some("Yes") => true,
            Some("No") | None => false,
            Some(other) => {
                return Err(format!(
                    "AppleClamshellState is {other:?}, expected Yes or No"
                ))
            }
        };
        let graphics_on = match property(output, "System Capabilities") {
            None => None,
            Some(mask) => {
                let mask = mask.parse::<u64>().map_err(|error| {
                    format!("System Capabilities {mask:?} is not a number: {error}")
                })?;
                Some(mask & CAPABILITY_GRAPHICS != 0)
            }
        };
        Ok(Self {
            lid_closed,
            graphics_on,
        })
    }

    /// The refusal for a closed lid. Nothing can wake a display behind one.
    pub fn require_lid_open(&self) -> Result<(), String> {
        if self.lid_closed {
            return Err(
                "the laptop lid is closed, so the game window cannot be shown and a run would fail \
                 after about 100 seconds with a misleading \"presentation was not acknowledged\" \
                 error. Open the lid and run again."
                    .into(),
            );
        }
        Ok(())
    }

    /// The refusal for a display that is asleep or off.
    pub fn require_display_on(&self) -> Result<(), String> {
        if self.graphics_on == Some(false) {
            return Err(
                "the display is asleep or off, so the game window cannot be shown. Wake the display \
                 and run again."
                    .into(),
            );
        }
        Ok(())
    }
}

/// The value of a top-level `"name" = value` line.
fn property<'a>(output: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("\"{name}\" = ");
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(prefix.as_str()))
        .map(str::trim)
}

fn read_host_power() -> Result<HostPower, String> {
    let output = Command::new(IOREG)
        .args(["-r", "-k", "AppleClamshellState", "-d", "1"])
        .output()
        .map_err(|error| format!("run {IOREG}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{IOREG} exited with {}", output.status));
    }
    HostPower::parse(&String::from_utf8_lossy(&output.stdout))
}

/// Refuse a Mac that cannot show the game window, then keep its display awake.
///
/// The lid is checked first because nothing can wake a display behind a closed
/// lid. The display is checked after the wake assertion is taken, so a display
/// that had merely idle-slept is woken instead of refused; one that stays off
/// is refused. Off macOS there is nothing to check.
pub fn require_ready_host() -> Result<DisplayAwake, String> {
    require_ready_host_with(
        cfg!(target_os = "macos"),
        &mut read_host_power,
        &mut hold_display_awake,
    )
}

fn require_ready_host_with(
    macos: bool,
    read: &mut dyn FnMut() -> Result<HostPower, String>,
    hold: &mut dyn FnMut() -> Result<DisplayAwake, String>,
) -> Result<DisplayAwake, String> {
    if !macos {
        return hold();
    }
    read()?.require_lid_open()?;
    let awake = hold()?;
    read()?.require_display_on()?;
    Ok(awake)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOSED_LID_DARK_WAKE: &str = r#"+-o IOPMrootDomain  <class IOPMrootDomain, id 0x1000002f6, registered, matched, active, busy 0 (3130 ms), retain 192>
    {
      "Wake Type" = "HID Activity"
      "IOPowerManagement" = {"ChildrenPowerState"=0,"DevicePowerState"=4,"CurrentPowerState"=4,"CapabilityFlags"=2,"MaxPowerState"=4}
      "System Capabilities" = 9
      "Last Sleep Reason" = "Clamshell Sleep"
      "AppleClamshellCausesSleep" = No
      "AppleClamshellState" = Yes
      "SleepDisabled" = No
    }
"#;

    fn open_lid(capabilities: u32) -> String {
        format!(
            "+-o IOPMrootDomain\n    {{\n      \"System Capabilities\" = {capabilities}\n      \
             \"AppleClamshellCausesSleep\" = Yes\n      \"AppleClamshellState\" = No\n    }}\n"
        )
    }

    #[test]
    fn closed_lid_and_dark_wake_are_read_from_a_captured_sample() {
        let power = HostPower::parse(CLOSED_LID_DARK_WAKE).unwrap();
        assert!(power.lid_closed);
        assert_eq!(power.graphics_on, Some(false));
    }

    #[test]
    fn open_lid_with_graphics_is_usable() {
        let power = HostPower::parse(&open_lid(15)).unwrap();
        assert!(!power.lid_closed);
        assert_eq!(power.graphics_on, Some(true));
        power.require_lid_open().unwrap();
        power.require_display_on().unwrap();
    }

    #[test]
    fn open_lid_without_graphics_is_a_sleeping_display() {
        let power = HostPower::parse(&open_lid(9)).unwrap();
        power.require_lid_open().unwrap();
        let refusal = power.require_display_on().unwrap_err();
        assert!(refusal.contains("display is asleep"), "{refusal}");
    }

    #[test]
    fn closed_lid_refusal_names_the_lid_and_the_misleading_error() {
        let refusal = HostPower::parse(CLOSED_LID_DARK_WAKE)
            .unwrap()
            .require_lid_open()
            .unwrap_err();
        assert!(refusal.contains("lid is closed"), "{refusal}");
        assert!(
            refusal.contains("presentation was not acknowledged"),
            "{refusal}"
        );
    }

    #[test]
    fn a_mac_without_a_lid_prints_nothing_and_is_not_refused() {
        let power = HostPower::parse("").unwrap();
        assert_eq!(
            power,
            HostPower {
                lid_closed: false,
                graphics_on: None
            }
        );
        power.require_lid_open().unwrap();
        power.require_display_on().unwrap();
    }

    #[test]
    fn unexpected_values_are_errors_not_guesses() {
        assert!(HostPower::parse("\"AppleClamshellState\" = Maybe\n").is_err());
        assert!(HostPower::parse("\"System Capabilities\" = lots\n").is_err());
    }

    #[test]
    fn a_closed_lid_is_refused_before_anything_is_woken() {
        let mut held = false;
        let result = require_ready_host_with(
            true,
            &mut || HostPower::parse(CLOSED_LID_DARK_WAKE),
            &mut || {
                held = true;
                Ok(DisplayAwake::inert())
            },
        );
        assert!(result.unwrap_err().contains("lid is closed"));
        assert!(!held, "the display must not be woken behind a closed lid");
    }

    #[test]
    fn a_display_that_stays_off_after_the_wake_is_refused() {
        let result =
            require_ready_host_with(true, &mut || HostPower::parse(&open_lid(9)), &mut || {
                Ok(DisplayAwake::inert())
            });
        assert!(result.unwrap_err().contains("display is asleep"));
    }

    #[test]
    fn off_macos_nothing_is_read() {
        let result =
            require_ready_host_with(false, &mut || Err("must not be read".into()), &mut || {
                Ok(DisplayAwake::inert())
            });
        assert!(result.is_ok());
    }
}
