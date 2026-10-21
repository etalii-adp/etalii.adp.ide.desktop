//! The colour of `color(c)` (DISL section 12.4; the standalone host's `DislColor`): a hex colour taken apart into
//! its sRGB channels and its alpha, each from 0 to 1, which `mix` manipulates and writes back as `#rrggbbaa`. Only
//! the hex forms are read (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`); any other colour is an evaluation error. Of the
//! colour's methods only `mix` is implemented, the one a bundled definition calls; `lighten`, `darken`, `alpha` and
//! `contrastText` are refused at load, naming them.

use std::any::Any;
use std::rc::Rc;

use adp_cel::{Object, Value};

/// A colour as CEL holds it: an opaque host value, which only `mix` takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DislColor {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

impl Object for DislColor {
    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_selectable(&self) -> bool {
        false
    }

    fn equals(&self, other: &dyn Object) -> bool {
        other.as_any().downcast_ref::<DislColor>() == Some(self)
    }

    fn text(&self) -> String {
        format!(
            "DislColor {{ Red = {}, Green = {}, Blue = {}, Alpha = {} }}",
            self.red, self.green, self.blue, self.alpha
        )
    }
}

impl DislColor {
    /// `color(c)`: `value` read as a hex colour.
    pub(crate) fn of(value: &Value) -> Result<DislColor, String> {
        let text = value.as_str().ok_or("color() takes a colour string.")?;
        let refused = || format!("color() takes a hex colour, not '{text}'.");
        let Some(hex) = text.strip_prefix('#') else {
            return Err(refused());
        };
        let mut hex: String = hex.to_owned();
        if matches!(hex.chars().count(), 3 | 4) {
            hex = hex.chars().flat_map(|d| [d, d]).collect();
        }
        if !matches!(hex.len(), 6 | 8) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(refused());
        }
        let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).map(|v| f64::from(v) / 255.0);
        let channel = |at: usize| channel(at).map_err(|_| refused());
        Ok(DislColor {
            red: channel(0)?,
            green: channel(2)?,
            blue: channel(4)?,
            alpha: if hex.len() == 8 { channel(6)? } else { 1.0 },
        })
    }

    /// `color(c).mix(c2, f)`: CSS Color 5 `color-mix(in srgb, c (1 − f) × 100%, c2 f × 100%)`, interpolated on
    /// gamma-encoded sRGB with premultiplied alpha and written as `#rrggbbaa`.
    pub(crate) fn mix(receiver: &Value, other: &Value, fraction: &Value) -> Result<Value, String> {
        let first = *receiver
            .downcast::<DislColor>()
            .ok_or("mix() is a method of color().")?;
        let second = DislColor::of(other)?;
        let f = match fraction {
            Value::Int(i) => *i as f64,
            Value::Double(d) => *d,
            _ => return Err("mix() takes a fraction from 0 to 1.".into()),
        };
        if !(0.0..=1.0).contains(&f) || f.is_nan() {
            return Err(format!(
                "mix() takes a fraction from 0 to 1, not {}.",
                adp_fbl::planning::dotnet_double(f)
            ));
        }
        let alpha = first.alpha * (1.0 - f) + second.alpha * f;
        let channel = |one: f64, two: f64| {
            if alpha == 0.0 {
                0.0
            } else {
                (one * first.alpha * (1.0 - f) + two * second.alpha * f) / alpha
            }
        };
        let channels = [
            channel(first.red, second.red),
            channel(first.green, second.green),
            channel(first.blue, second.blue),
            alpha,
        ];
        let mut text = String::from("#");
        for c in channels {
            text.push_str(&format!("{:02x}", (c.clamp(0.0, 1.0) * 255.0).round() as i64));
        }
        Ok(Value::String(text))
    }
}
