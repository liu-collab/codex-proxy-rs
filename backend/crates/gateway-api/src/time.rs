//! 管理与自助页面共用的时间展示投影

use chrono::{DateTime, Utc};
use gateway_core::time::DeploymentTimeZone;

pub(crate) fn query_range(
    start: Option<&str>,
    end: Option<&str>,
    period: Option<&str>,
    as_of: Option<i64>,
    timezone: DeploymentTimeZone,
    default_period: &str,
) -> Result<gateway_admin::model::observability::TimeRange, crate::admin::WireValidationError> {
    use crate::admin::{WireValidationError, observability::parse_datetime};
    use gateway_admin::model::observability::{CalendarPeriod, TimeRange};
    if start.is_some() || end.is_some() {
        if period.is_some() || as_of.is_some() {
            return Err(WireValidationError::new("timeRange"));
        }
        let end = parse_datetime(end)?.unwrap_or_else(Utc::now);
        let start = match parse_datetime(start)? {
            Some(start) => start,
            None => {
                let period = CalendarPeriod::parse(default_period)
                    .map_err(|_| WireValidationError::new("period"))?;
                return TimeRange::calendar_at(period, end, timezone)
                    .map_err(|_| WireValidationError::new("timeRange"));
            }
        };
        return TimeRange::new(start, end).map_err(|_| WireValidationError::new("timeRange"));
    }
    let end = match as_of {
        Some(value) => DateTime::from_timestamp_millis(value)
            .ok_or_else(|| WireValidationError::new("asOf"))?,
        None => Utc::now(),
    };
    let period = CalendarPeriod::parse(period.unwrap_or(default_period))
        .map_err(|_| WireValidationError::new("period"))?;
    TimeRange::calendar_at(period, end, timezone).map_err(|_| WireValidationError::new("timeRange"))
}

#[derive(Clone, Copy)]
pub struct TimePresenter {
    timezone: DeploymentTimeZone,
    now: DateTime<Utc>,
}

impl TimePresenter {
    pub(crate) fn today(self) -> chrono::NaiveDate {
        self.timezone.local(self.now).date_naive()
    }
    pub(crate) fn now(self) -> DateTime<Utc> {
        self.now
    }
    pub(crate) fn rfc_display(self, value: Option<&str>) -> Option<String> {
        value
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| self.datetime(&value.to_utc()))
    }
    pub(crate) fn request_buckets(
        self,
        buckets: impl Iterator<Item = (DateTime<Utc>, u64)>,
    ) -> Vec<RequestBucketView> {
        buckets
            .map(|(start, count)| {
                let end = start + chrono::Duration::hours(1);
                RequestBucketView {
                    bucket_start: start,
                    request_count: count,
                    label: format!(
                        "{}–{} · {count} 次请求",
                        self.label(start, "%m-%d %H:%M"),
                        self.label(end, "%H:%M")
                    ),
                }
            })
            .collect()
    }

    #[must_use]
    pub fn new(timezone: DeploymentTimeZone) -> Self {
        Self {
            timezone,
            now: Utc::now(),
        }
    }

    pub(crate) fn datetime(self, value: &DateTime<Utc>) -> String {
        self.label(*value, "%Y-%m-%d %H:%M:%S")
    }

    pub(crate) fn time(self, value: &DateTime<Utc>) -> String {
        self.label(*value, "%H:%M:%S")
    }

    pub(crate) fn label(self, value: DateTime<Utc>, format: &str) -> String {
        self.timezone.local(value).format(format).to_string()
    }

    pub(crate) fn rfc3339(self, value: &DateTime<Utc>) -> String {
        self.timezone.local(*value).to_rfc3339()
    }

    pub(crate) fn relative_view(
        self,
        value: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> RelativeTimeView {
        let elapsed = now.signed_duration_since(value);
        if elapsed.num_seconds() < 0 {
            return RelativeTimeView::Absolute {
                value: self.datetime(&value),
            };
        }
        if elapsed.num_seconds() < 60 {
            return RelativeTimeView::JustNow;
        }
        if elapsed.num_minutes() < 60 {
            return RelativeTimeView::Elapsed {
                value: elapsed.num_minutes(),
                unit: RelativeTimeUnit::Minute,
            };
        }
        if elapsed.num_hours() < 24 {
            return RelativeTimeView::Elapsed {
                value: elapsed.num_hours(),
                unit: RelativeTimeUnit::Hour,
            };
        }
        RelativeTimeView::Elapsed {
            value: elapsed.num_days(),
            unit: RelativeTimeUnit::Day,
        }
    }

    pub(crate) fn relative(self, value: DateTime<Utc>, now: DateTime<Utc>) -> String {
        match self.relative_view(value, now) {
            RelativeTimeView::JustNow => "刚刚".to_owned(),
            RelativeTimeView::Elapsed { value, unit } => {
                let unit = match unit {
                    RelativeTimeUnit::Minute => "分钟",
                    RelativeTimeUnit::Hour => "小时",
                    RelativeTimeUnit::Day => "天",
                };
                format!("{value} {unit}前")
            }
            RelativeTimeView::Absolute { value } => value,
        }
    }

    pub(crate) fn relative_optional(
        self,
        value: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> String {
        value.map_or_else(|| "—".to_owned(), |value| self.relative(value, now))
    }
}

/// 后端按查询锚点与部署时区决定展示方式，前端只本地化分类和单位
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelativeTimeView {
    JustNow,
    Elapsed { value: i64, unit: RelativeTimeUnit },
    Absolute { value: String },
}

/// 相对时间保持分钟、小时、天的固定精度，不引入月或年的日历推算
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelativeTimeUnit {
    Minute,
    Hour,
    Day,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestBucketView {
    pub(crate) bucket_start: DateTime<Utc>,
    pub(crate) request_count: u64,
    pub(crate) label: String,
}
