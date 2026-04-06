pub mod bbox;

pub use bbox::BBox;

// ---------------------------------------------------------------------------
// Math helpers (Houdini-style)
// ---------------------------------------------------------------------------

/// Remap `value` from [old_min, old_max] to [new_min, new_max].
/// Does not clamp — values outside the input range extrapolate linearly.
pub fn fit(value: f32, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> f32 {
    let old_range = old_max - old_min;
    if old_range == 0.0 {
        return new_min;
    }
    let t = (value - old_min) / old_range;
    new_min + t * (new_max - new_min)
}

/// Clamped remap: like `fit` but the output is clamped to [new_min, new_max].
pub fn efit(value: f32, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> f32 {
    let result = fit(value, old_min, old_max, new_min, new_max);
    let lo = new_min.min(new_max);
    let hi = new_min.max(new_max);
    result.clamp(lo, hi)
}

/// Hermite smooth-step: maps `value` in [min, max] smoothly to [0, 1].
/// Values outside [min, max] are clamped.
pub fn smooth(value: f32, min: f32, max: f32) -> f32 {
    let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn fit_basic() {
        // Map 0.5 from [0,1] to [0,100]
        assert_relative_eq!(fit(0.5, 0.0, 1.0, 0.0, 100.0), 50.0);
        // Extrapolate beyond range
        assert_relative_eq!(fit(2.0, 0.0, 1.0, 0.0, 100.0), 200.0);
        // Reverse mapping
        assert_relative_eq!(fit(0.0, -1.0, 1.0, 10.0, 20.0), 15.0);
    }

    #[test]
    fn efit_clamps() {
        // Within range — same as fit
        assert_relative_eq!(efit(0.5, 0.0, 1.0, 0.0, 100.0), 50.0);
        // Above range — clamped to new_max
        assert_relative_eq!(efit(2.0, 0.0, 1.0, 0.0, 100.0), 100.0);
        // Below range — clamped to new_min
        assert_relative_eq!(efit(-1.0, 0.0, 1.0, 0.0, 100.0), 0.0);
    }

    #[test]
    fn smooth_hermite() {
        // At boundaries
        assert_relative_eq!(smooth(0.0, 0.0, 1.0), 0.0);
        assert_relative_eq!(smooth(1.0, 0.0, 1.0), 1.0);
        // Midpoint
        assert_relative_eq!(smooth(0.5, 0.0, 1.0), 0.5);
        // Clamping outside range
        assert_relative_eq!(smooth(-1.0, 0.0, 1.0), 0.0);
        assert_relative_eq!(smooth(2.0, 0.0, 1.0), 1.0);
    }
}
