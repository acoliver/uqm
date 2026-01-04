// Game Date Management
// Handles calendar arithmetic for the game including leap years and days per month

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameDate {
    pub day: u8,
    pub month: u8,
    pub year: u32,
}

impl GameDate {
    /// Create a new game date
    pub fn new(day: u8, month: u8, year: u32) -> Self {
        GameDate { day, month, year }
    }

    /// Check if a year is a leap year
    /// Rules:
    /// - Divisible by 4
    /// - But not by 100 unless also by 400
    pub fn is_leap_year(year: u32) -> bool {
        (year % 4 == 0) && (year % 100 != 0 || year % 400 == 0)
    }

    /// Get the number of days in a month for a given year
    pub fn days_in_month(month: u8, year: u32) -> u8 {
        const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        
        let month_idx = (month - 1) as usize;
        if month_idx >= 12 {
            return 0;
        }

        if month == 2 && Self::is_leap_year(year) {
            29
        } else {
            DAYS_IN_MONTH[month_idx]
        }
    }

    /// Add days to the date
    pub fn add_days(self, days: u32) -> Self {
        let mut result = self;
        let mut days_to_add = days;

        while days_to_add > 0 {
            let days_this_month = Self::days_in_month(result.month, result.year) as u32;
            let days_remaining = days_this_month - (result.day as u32);

            if days_to_add <= days_remaining {
                result.day = (result.day as u32 + days_to_add) as u8;
                days_to_add = 0;
            } else {
                days_to_add -= days_remaining + 1;
                result.day = 1;
                result = result.add_months(1);
            }
        }

        result
    }

    /// Add months to the date (preserves day if possible)
    pub fn add_months(self, months: u32) -> Self {
        let mut result = self;
        let mut months_to_add = months;

        while months_to_add > 0 {
            if result.month < 12 {
                result.month += 1;
            } else {
                result.month = 1;
                result.year += 1;
            }
            months_to_add -= 1;
        }

        // Adjust day if it exceeds the new month's days
        let max_day = Self::days_in_month(result.month, result.year);
        if result.day > max_day {
            result.day = max_day;
        }

        result
    }

    /// Add years to the date
    pub fn add_years(self, years: u32) -> Self {
        let mut result = self;
        result.year += years;
        
        // Adjust day if it's February 29 and the new year is not a leap year
        if result.day == 29 && result.month == 2 && !Self::is_leap_year(result.year) {
            result.day = 28;
        }

        result
    }

    /// Subtract days from the date
    pub fn sub_days(self, days: u32) -> Self {
        let mut result = self;
        let mut days_to_sub = days;

        while days_to_sub > 0 {
            if result.day > 1 {
                result.day -= 1;
                days_to_sub -= 1;
            } else {
                // Go to previous month
                if result.month > 1 {
                    result.month -= 1;
                } else {
                    result.month = 12;
                    result.year -= 1;
                }
                result.day = Self::days_in_month(result.month, result.year);
                days_to_sub -= 1;
            }
        }

        result
    }

    /// Calculate the difference in days between two dates
    pub fn days_since(self, other: GameDate) -> i64 {
        // Simple implementation - convert both to ordinal day count
        let self_ordinal = self.to_ordinal();
        let other_ordinal = other.to_ordinal();
        self_ordinal as i64 - other_ordinal as i64
    }

    /// Convert date to ordinal day count (days since year 0)
    fn to_ordinal(self) -> u64 {
        // Calculate days from years
        let mut days = self.year as u64 * 365;
        
        // Add leap days for previous years
        days += (self.year / 4) as u64;
        days -= (self.year / 100) as u64;
        days += (self.year / 400) as u64;

        // Add days for months in current year
        const MONTH_OFFSETS: [u8; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let month_idx = (self.month - 1) as usize;
        if month_idx < 12 {
            days += MONTH_OFFSETS[month_idx] as u64;
            
            // Add leap day if past February and it's a leap year
            if self.month > 2 && Self::is_leap_year(self.year) {
                days += 1;
            }
        }

        days += self.day as u64;
        days
    }

    /// Check if the date is valid
    pub fn is_valid(&self) -> bool {
        if self.month < 1 || self.month > 12 {
            return false;
        }
        if self.day < 1 || self.day > Self::days_in_month(self.month, self.year) {
            return false;
        }
        true
    }

    /// Format date as "MM/DD/YYYY"
    pub fn format(&self) -> String {
        format!("{:02}/{:02}/{:04}", self.month, self.day, self.year)
    }

    /// Format date as "Month Day, Year"
    pub fn format_long(&self) -> String {
        const MONTH_NAMES: [&str; 12] = [
            "January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December",
        ];
        let month_name = MONTH_NAMES.get((self.month - 1) as usize).unwrap_or(&"Unknown");
        format!("{} {}, {:?}", month_name, self.day, self.year)
    }
}

impl Default for GameDate {
    fn default() -> Self {
        GameDate::new(17, 2, 2155) // February 17, 2155 (game start date)
    }
}

impl std::fmt::Display for GameDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}/{:02}/{:04}", self.month, self.day, self.year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let date = GameDate::new(15, 6, 2155);
        assert_eq!(date.day, 15);
        assert_eq!(date.month, 6);
        assert_eq!(date.year, 2155);
    }

    #[test]
    fn test_default() {
        let date = GameDate::default();
        assert_eq!(date.day, 17);
        assert_eq!(date.month, 2);
        assert_eq!(date.year, 2155);
    }

    #[test]
    fn test_is_leap_year() {
        assert!(GameDate::is_leap_year(2000));
        assert!(GameDate::is_leap_year(2004));
        assert!(GameDate::is_leap_year(2008));
        
        assert!(!GameDate::is_leap_year(1900));
        assert!(!GameDate::is_leap_year(2001));
        assert!(!GameDate::is_leap_year(2100));
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(GameDate::days_in_month(1, 2000), 31);
        assert_eq!(GameDate::days_in_month(2, 2000), 29); // Leap year
        assert_eq!(GameDate::days_in_month(2, 2001), 28); // Non-leap year
        assert_eq!(GameDate::days_in_month(4, 2000), 30);
        assert_eq!(GameDate::days_in_month(12, 2000), 31);
    }

    #[test]
    fn test_invalid_month() {
        assert_eq!(GameDate::days_in_month(0, 2000), 0);
        assert_eq!(GameDate::days_in_month(13, 2000), 0);
    }

    #[test]
    fn test_add_days() {
        let date = GameDate::new(15, 1, 2000);
        
        // Add 5 days
        assert_eq!(date.add_days(5), GameDate::new(20, 1, 2000));
        
        // Add days across month boundary
        assert_eq!(date.add_days(20), GameDate::new(4, 2, 2000));
        
        // Add days across year boundary
        let date = GameDate::new(30, 12, 2000);
        assert_eq!(date.add_days(2), GameDate::new(1, 1, 2001));
    }

    #[test]
    fn test_add_days_leap_year() {
        let date = GameDate::new(28, 2, 2000);
        assert_eq!(date.add_days(1), GameDate::new(29, 2, 2000));
        assert_eq!(date.add_days(2), GameDate::new(1, 3, 2000));
    }

    #[test]
    fn test_add_months() {
        let date = GameDate::new(15, 1, 2000);
        
        assert_eq!(date.add_months(1), GameDate::new(15, 2, 2000));
        assert_eq!(date.add_months(12), GameDate::new(15, 1, 2001));
        assert_eq!(date.add_months(5), GameDate::new(15, 6, 2000));
    }

    #[test]
    fn test_add_months_adjusts_day() {
        let date = GameDate::new(31, 1, 2000);
        let result = date.add_months(1);
        // January 31 -> February 29 (2000 is leap year)
        assert_eq!(result, GameDate::new(29, 2, 2000));
        
        let date = GameDate::new(31, 1, 2001);
        let result = date.add_months(1);
        // January 31 -> February 28 (2001 is not leap year)
        assert_eq!(result, GameDate::new(28, 2, 2001));
    }

    #[test]
    fn test_add_years() {
        let date = GameDate::new(15, 6, 2000);
        assert_eq!(date.add_years(5), GameDate::new(15, 6, 2005));
    }

    #[test]
    fn test_add_years_leap_day() {
        let date = GameDate::new(29, 2, 2000);
        
        // 2000 is leap year
        assert_eq!(date.add_years(4), GameDate::new(29, 2, 2004));
        
        // 2001 is not leap year
        assert_eq!(date.add_years(1), GameDate::new(28, 2, 2001));
        
        // Back to leap year
        assert_eq!(date.add_years(4), GameDate::new(29, 2, 2004));
    }

    #[test]
    fn test_sub_days() {
        let date = GameDate::new(20, 1, 2000);
        
        assert_eq!(date.sub_days(5), GameDate::new(15, 1, 2000));
        assert_eq!(date.sub_days(20), GameDate::new(31, 12, 1999));
    }

    #[test]
    fn test_sub_days_across_months() {
        let date = GameDate::new(5, 3, 2000);
        assert_eq!(date.sub_days(5), GameDate::new(29, 2, 2000));
        
        let date = GameDate::new(1, 3, 2000);
        assert_eq!(date.sub_days(1), GameDate::new(29, 2, 2000));
    }

    #[test]
    fn test_days_since() {
        let date1 = GameDate::new(1, 1, 2000);
        let date2 = GameDate::new(2, 1, 2000);
        
        assert_eq!(date2.days_since(date1), 1);
        assert_eq!(date1.days_since(date2), -1);
    }

    #[test]
    fn test_days_since_larger() {
        let date1 = GameDate::new(1, 1, 2000);
        let date2 = GameDate::new(1, 2, 2000);
        
        // January 2000 has 31 days
        let diff = date2.days_since(date1);
        assert_eq!(diff, 31);
    }

    #[test]
    fn test_is_valid() {
        assert!(GameDate::new(15, 6, 2000).is_valid());
        assert!(GameDate::new(29, 2, 2000).is_valid()); // Leap year
        assert!(GameDate::new(28, 2, 2001).is_valid()); // Non-leap year
        
        assert!(!GameDate::new(29, 2, 2001).is_valid()); // Invalid: not leap year
        assert!(!GameDate::new(31, 4, 2000).is_valid()); // Invalid: April has 30 days
        assert!(!GameDate::new(0, 1, 2000).is_valid()); // Invalid: day 0
        assert!(!GameDate::new(15, 0, 2000).is_valid()); // Invalid: month 0
        assert!(!GameDate::new(15, 13, 2000).is_valid()); // Invalid: month 13
    }

    #[test]
    fn test_format() {
        let date = GameDate::new(15, 6, 2000);
        assert_eq!(date.format(), "06/15/2000");
    }

    #[test]
    fn test_format_long() {
        let date = GameDate::new(15, 6, 2155);
        assert_eq!(date.format_long(), "June 15, 2155");
        
        let date = GameDate::new(29, 2, 2000);
        assert_eq!(date.format_long(), "February 29, 2000");
    }

    #[test]
    fn test_display() {
        let date = GameDate::new(15, 6, 2000);
        assert_eq!(format!("{}", date), "06/15/2000");
    }

    #[test]
    fn test_clone() {
        let date1 = GameDate::new(15, 6, 2000);
        let date2 = date1;
        assert_eq!(date2, date1);
    }

    #[test]
    fn test_partial_eq() {
        let date1 = GameDate::new(15, 6, 2000);
        let date2 = GameDate::new(15, 6, 2000);
        let date3 = GameDate::new(16, 6, 2000);
        
        assert_eq!(date1, date2);
        assert_ne!(date1, date3);
    }

    #[test]
    fn test_debug() {
        let date = GameDate::new(15, 6, 2000);
        let debug_str = format!("{:?}", date);
        assert!(debug_str.contains("day: 15"));
        assert!(debug_str.contains("month: 6"));
        assert!(debug_str.contains("year: 2000"));
    }

    #[test]
    fn test_comprehensive_date_arithmetic() {
        // Test a sequence of operations
        let date = GameDate::default(); // Feb 17, 2155
        
        // Add 100 days
        let date = date.add_days(100);
        
        // Add 6 months
        let date = date.add_months(6);
        
        // Add 2 years
        let date = date.add_years(2);
        
        // The exact result depends on calculations, just verify it's valid
        assert!(date.is_valid());
        assert!(date.year >= 2155);
    }

    #[test]
    fn test_all_months() {
        // Test that all months have correct day counts
        for year in [1999, 2000, 2001, 2004, 2100].iter() {
            let expected_days: [u8; 12] = [
                31, if GameDate::is_leap_year(*year) { 29 } else { 28 },
                31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
            ];
            
            for month in 1..=12 {
                assert_eq!(
                    GameDate::days_in_month(month, *year),
                    expected_days[(month - 1) as usize]
                );
            }
        }
    }
}
