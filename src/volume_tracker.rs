use chrono::{DateTime, Utc};
use std::collections::VecDeque;

/// Individual swap record with timestamp
#[derive(Debug, Clone)]
pub struct SwapRecord {
    pub timestamp: DateTime<Utc>,
    pub volume_sol: f64,
}

/// Volume tracker with rolling 24-hour window
/// Automatically expires old swaps and tracks volume/count metrics
#[derive(Debug, Clone)]
pub struct VolumeTracker {
    swaps: VecDeque<SwapRecord>,
}

impl VolumeTracker {
    pub fn new() -> Self {
        Self {
            swaps: VecDeque::new(),
        }
    }

    /// Add a new swap and remove old ones outside 24h window
    pub fn add_swap(&mut self, volume_sol: f64) {
        let now = Utc::now();
        let cutoff = now - chrono::Duration::hours(24);

        // Remove swaps older than 24 hours
        while let Some(oldest) = self.swaps.front() {
            if oldest.timestamp < cutoff {
                self.swaps.pop_front();
            } else {
                break;
            }
        }

        // Add new swap
        self.swaps.push_back(SwapRecord {
            timestamp: now,
            volume_sol,
        });
    }

    /// Get total volume in last 24 hours
    pub fn get_24h_volume(&self) -> f64 {
        self.swaps.iter().map(|s| s.volume_sol).sum()
    }

    /// Get number of swaps in last 24 hours
    pub fn get_swap_count(&self) -> usize {
        self.swaps.len()
    }

    /// Remove swaps older than 24 hours (called periodically for cleanup)
    pub fn cleanup_old_swaps(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        while let Some(oldest) = self.swaps.front() {
            if oldest.timestamp < cutoff {
                self.swaps.pop_front();
            } else {
                break;
            }
        }
    }
}

impl Default for VolumeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_tracker() {
        let mut tracker = VolumeTracker::new();

        // Add some swaps
        tracker.add_swap(1.0);
        tracker.add_swap(0.5);
        tracker.add_swap(2.0);

        assert_eq!(tracker.get_swap_count(), 3);
        assert_eq!(tracker.get_24h_volume(), 3.5);
    }

    #[test]
    fn test_volume_tracker_cleanup() {
        let mut tracker = VolumeTracker::new();

        // Add swaps
        tracker.add_swap(1.0);
        tracker.add_swap(2.0);

        assert_eq!(tracker.get_swap_count(), 2);

        // Cleanup shouldn't remove recent swaps
        tracker.cleanup_old_swaps();
        assert_eq!(tracker.get_swap_count(), 2);
    }
}
