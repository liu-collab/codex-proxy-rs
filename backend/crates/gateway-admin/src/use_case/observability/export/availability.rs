//! 集中归纳已采集证据的缺口，不从缺失事件推断业务结果

use serde_json::Value;

use crate::model::diagnostics::DiagnosticBundle;

use super::gap;

pub(super) fn trace_gaps(bundle: &mut DiagnosticBundle, section: &str, trace: &Value) {
    let trace_section = format!("{section}.trace");
    for (field, reason) in [
        ("droppedEvents", "events_evicted"),
        ("droppedEvidenceEvents", "critical_evidence_evicted"),
        ("stageCountOverflow", "stage_coverage_incomplete"),
    ] {
        if trace[field].as_u64().unwrap_or(0) > 0 {
            gap(bundle, &trace_section, reason);
        }
    }
    if !trace["stageCounts"].is_object() {
        gap(bundle, &trace_section, "stage_coverage_not_recorded");
    }
    let events = trace["events"].as_array();
    if !events.is_some_and(|events| {
        events.iter().any(|event| {
            event["stage"] == "request.routed" && event["data"]["accountPolicy"].is_object()
        })
    }) {
        gap(bundle, section, "incident_policy_not_recorded");
    }
    for (index, event) in events.into_iter().flatten().enumerate() {
        let event_section = format!("{trace_section}.events[{index}]");
        let data = &event["data"];
        for (missing, reason) in [
            (
                event["stage"] == "provider.response.parse_gap",
                "response_parse_failed",
            ),
            (data["jsonValid"] == false, "json_parse_failed"),
            (data["truncated"] == true, "event_data_truncated"),
            (
                data["input"]["omittedItems"].as_u64().unwrap_or(0) > 0,
                "input_prefix_sample_limited",
            ),
            (
                data["correlation"].as_object().is_some_and(|headers| {
                    headers
                        .values()
                        .any(|header| header["omittedValues"].as_u64().unwrap_or(0) > 0)
                }),
                "correlation_header_sample_limited",
            ),
        ] {
            if missing {
                gap(bundle, &event_section, reason);
            }
        }
        if event["stage"] == "capture.gap" || data["truncated"] == true {
            gap(bundle, &trace_section, "capture_gap_or_truncation");
        }
        if event["stage"] == "retry.decided" && data["phase"].is_null() {
            gap(bundle, &trace_section, "legacy_retry_facts_not_recorded");
        }
    }
}
