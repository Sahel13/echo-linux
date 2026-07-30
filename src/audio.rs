use crate::settings::Microphone;
use cpal::traits::{DeviceTrait, HostTrait};
use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputDevice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceListError;

/// Enumerate the inputs that are available from the active audio host.
///
/// CPAL does not expose a separate stable device identifier on Linux, so the
/// device name is retained as the selection identifier. This is sufficient to
/// restore a selection after a device is temporarily unavailable, while still
/// permitting the UI to fall back safely when that name disappears.
pub fn list_input_devices() -> Result<Vec<InputDevice>, DeviceListError> {
    let host = cpal::default_host();
    let devices = host.input_devices().map_err(|_| DeviceListError)?;
    let mut seen = HashSet::new();
    let mut inputs = devices
        .filter_map(|device| {
            let name = device.name().ok()?;
            seen.insert(name.clone()).then_some(InputDevice {
                id: name.clone(),
                name,
            })
        })
        .collect::<Vec<_>>();
    inputs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(inputs)
}

pub fn selected_index(selection: &Microphone, devices: &[InputDevice]) -> Option<u32> {
    match selection {
        Microphone::SystemDefault => Some(0),
        Microphone::Device { id } => devices
            .iter()
            .position(|device| &device.id == id)
            .and_then(|index| u32::try_from(index + 1).ok()),
    }
}

/// Return the currently usable selection and whether a missing device forced a
/// fallback to the system default.
pub fn reconcile_selection(selection: &Microphone, devices: &[InputDevice]) -> (Microphone, bool) {
    if selected_index(selection, devices).is_some() {
        (selection.clone(), false)
    } else {
        (Microphone::SystemDefault, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn devices() -> Vec<InputDevice> {
        vec![
            InputDevice {
                id: "Built-in Microphone".into(),
                name: "Built-in Microphone".into(),
            },
            InputDevice {
                id: "USB Microphone".into(),
                name: "USB Microphone".into(),
            },
        ]
    }

    #[test]
    fn system_default_is_always_the_first_input_choice() {
        assert_eq!(
            selected_index(&Microphone::SystemDefault, &devices()),
            Some(0)
        );
    }

    #[test]
    fn connected_selected_microphone_keeps_its_choice() {
        let selection = Microphone::Device {
            id: "USB Microphone".into(),
        };
        assert_eq!(selected_index(&selection, &devices()), Some(2));
        assert_eq!(
            reconcile_selection(&selection, &devices()),
            (selection, false)
        );
    }

    #[test]
    fn disappeared_microphone_falls_back_to_system_default() {
        let selection = Microphone::Device {
            id: "USB Microphone".into(),
        };
        let connected = vec![devices()[0].clone()];
        assert_eq!(
            reconcile_selection(&selection, &connected),
            (Microphone::SystemDefault, true)
        );
    }
}
