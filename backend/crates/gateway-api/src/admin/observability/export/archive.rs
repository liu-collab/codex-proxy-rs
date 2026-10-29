//! 只打包已经脱敏的诊断文档，摘要与正文使用同一份证据

use std::io::{self, Cursor, Write};

use serde_json::Value;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

pub(super) fn build(document: &Value) -> io::Result<Vec<u8>> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(1))
        .unix_permissions(0o600);
    archive.start_file("diagnostics.json", options)?;
    serde_json::to_writer_pretty(&mut archive, document)?;
    archive.start_file("summary.txt", options)?;
    write!(
        archive,
        "CPR 诊断摘要\n导出时间：{}\n目标：{}\n导出截断：{}\n环境：{}\n",
        document["collectedAt"],
        document["target"],
        document["exportTruncated"],
        document["environment"]
    )?;
    for request in document["requests"].as_array().into_iter().flatten() {
        let evidence = &request["evidence"];
        let trace = &evidence["trace"];
        let detail = &evidence["detail"]["request"];
        writeln!(
            archive,
            "\n结果：{}；状态码：{}；尝试数：{}\n模型：请求={}，发送={}\n传输：{} → {}\n时延：{}",
            detail["logicalOutcome"],
            detail["statusCode"],
            detail["attemptCount"],
            detail["requestedModel"],
            detail["upstreamModel"],
            detail["clientTransport"],
            detail["upstreamTransport"],
            detail["latencyDetails"]
        )?;
        writeln!(
            archive,
            "\n请求：{}\n总事件：{}；淘汰事件：{}；关键证据淘汰：{}\n采集阶段：{}",
            request["requestId"],
            trace["totalEvents"],
            trace["droppedEvents"],
            trace["droppedEvidenceEvents"],
            trace["stageCounts"]
        )?;
        for event in trace["events"].as_array().into_iter().flatten() {
            if matches!(
                event["stage"].as_str(),
                Some(
                    "request.finished"
                        | "attempt.failed"
                        | "retry.decided"
                        | "provider.response.usage"
                        | "request.usage"
                        | "capture.gap"
                        | "provider.response.parse_gap"
                )
            ) {
                writeln!(
                    archive,
                    "attempt={} t={}ms {}：{}",
                    event["attemptIndex"], event["elapsedMs"], event["stage"], event["data"]
                )?;
            }
        }
    }
    if !document["error"].is_null() {
        writeln!(archive, "\n目标错误：{}", document["error"])?;
    }
    writeln!(archive, "\n证据缺口：")?;
    for gap in document["availability"]["gaps"]
        .as_array()
        .into_iter()
        .flatten()
    {
        writeln!(archive, "{}：{}", gap["section"], gap["reason"])?;
    }
    archive.start_file("README.txt", options)?;
    archive.write_all("diagnostics.json 为完整结构化证据，summary.txt 为同一证据的摘要\n先检查 availability.gaps、exportTruncated 与 trace.stageCounts\n缺失、字段无效、上游明确返回 0、采集截断和证据淘汰含义不同\nprovider.response.usage 为上游终态观测及共享解析结果，request.usage 为网关最终归集结果\n输入指纹来自已解析对象；outbound 为准备发送的对象，成功发送需结合发送事实与响应判断\n指纹只能比较内容与顺序，不能证明上游缓存节点或缓存是否仍存在\nenvironment 是导出时环境；事发配置见 incidentConfiguration 与 request.routed\n历史记录不会补生成当时未采集的信息，未知业务内容不属于自动脱敏保证范围\n".as_bytes())?;
    Ok(archive.finish()?.into_inner())
}
