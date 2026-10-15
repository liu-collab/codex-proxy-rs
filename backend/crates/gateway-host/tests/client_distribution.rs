//! 验证客户端下载信息的版本选择、过期处理与官方地址回退

use chrono::{TimeZone as _, Utc};
use gateway_admin::model::client_distribution::{
    ClientArchitecture, ClientDownloadPackage, ClientDownloadSource,
};
use gateway_host::client_distribution::{
    ClientDistributionResolutionErrorKind, complete_with_official_fallback, parse_store_packages,
};

#[test]
fn parser_selects_latest_valid_package_for_each_architecture() {
    let now = Utc
        .with_ymd_and_hms(2026, 9, 1, 0, 0, 0)
        .single()
        .expect("time");
    let html = r#"
        <table class="tftable">
          <tr><td><a href="http://tlu.dl.delivery.mp.microsoft.com/filestreamingservice/files/old?x=1">OpenAI.Codex_26.800.1.0_x64__2p2nqsd0c76g0.msix</a></td><td>2026-09-01 01:00:00 GMT</td><td>700 MB</td></tr>
          <tr><td><a href="http://tlu.dl.delivery.mp.microsoft.com/filestreamingservice/files/new?x=1">OpenAI.Codex_26.825.6671.0_x64__2p2nqsd0c76g0.msix</a></td><td>2026-09-01 02:00:00 GMT</td><td>744.25 MB</td></tr>
          <tr><td><a href="https://dl.delivery.mp.microsoft.com/filestreamingservice/files/arm?x=1">OpenAI.Codex_26.825.6671.0_arm64__2p2nqsd0c76g0.msix</a></td><td>2026-09-01 02:00:00 GMT</td><td>710 MB</td></tr>
          <tr><td><a href="https://evil.example/filestreamingservice/files/bad">OpenAI.Codex_99.0.0.0_x64__2p2nqsd0c76g0.msix</a></td><td>2026-09-01 02:00:00 GMT</td><td>1 GB</td></tr>
          <tr><td><a href="https://dl.delivery.mp.microsoft.com/filestreamingservice/files/map">OpenAI.Codex_26.825.6671.0_x64__2p2nqsd0c76g0.BlockMap</a></td><td>2026-09-01 02:00:00 GMT</td><td>1 MB</td></tr>
        </table>
    "#;

    let packages = parse_store_packages(html, now).expect("packages");

    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0].architecture, ClientArchitecture::X64);
    assert_eq!(packages[0].version.as_deref(), Some("26.825.6671.0"));
    assert!(packages[0].download_url.starts_with("http://"));
    assert_eq!(packages[0].size_bytes, Some(780_402_688));
    assert_eq!(packages[1].architecture, ClientArchitecture::Arm64);
    assert!(packages[1].download_url.starts_with("https://"));
}

#[test]
fn parser_rejects_expiring_results() {
    let now = Utc
        .with_ymd_and_hms(2026, 9, 1, 0, 0, 0)
        .single()
        .expect("time");
    let html = r#"
        <table class="tftable">
          <tr><td><a href="https://dl.delivery.mp.microsoft.com/filestreamingservice/files/x64">OpenAI.Codex_26.825.6671.0_x64__2p2nqsd0c76g0.msix</a></td><td>2026-09-01 00:09:59 GMT</td><td>700 MB</td></tr>
        </table>
    "#;

    assert_eq!(
        parse_store_packages(html, now),
        Err(ClientDistributionResolutionErrorKind::InvalidResponse)
    );
}

#[test]
fn missing_architecture_uses_only_its_official_fallback() {
    let dynamic = ClientDownloadPackage {
        architecture: ClientArchitecture::X64,
        source: ClientDownloadSource::MicrosoftStore,
        version: Some("26.825.6671.0".to_owned()),
        file_name: "store-x64.msix".to_owned(),
        size_bytes: None,
        download_url: "https://dl.delivery.mp.microsoft.com/filestreamingservice/files/x64"
            .to_owned(),
        expires_at: None,
    };

    let (packages, warning) = complete_with_official_fallback(vec![dynamic]);

    assert_eq!(packages[0].source, ClientDownloadSource::MicrosoftStore);
    assert_eq!(packages[1].source, ClientDownloadSource::OfficialOpenAi);
    assert!(warning.is_some_and(|value| value.contains("arm64")));
}

#[tokio::test]
async fn official_package_size_reads_head_length_without_downloading_the_body() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/ChatGPT-x64.msix", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).await.unwrap();
            assert!(read > 0);
            request.extend_from_slice(&buffer[..read]);
        }
        assert!(request.starts_with(b"HEAD /ChatGPT-x64.msix HTTP/1.1\r\n"));
        // 只发送响应头，不发送大包正文，证明解析使用文件长度而不是 HEAD 正文长度
        stream
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 944929441\r\nconnection: close\r\n\r\n")
            .await
            .unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    assert_eq!(
        gateway_host::client_distribution::official_package_size(&client, &url).await,
        Some(944_929_441)
    );
    server.await.unwrap();
}

#[tokio::test]
async fn official_package_size_failure_keeps_metadata_optional() {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    let server = MockServer::start().await;
    for (route, response) in [
        ("/missing.msix", ResponseTemplate::new(404)),
        ("/empty.msix", ResponseTemplate::new(200)),
        (
            "/slow.msix",
            ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(4)),
        ),
    ] {
        Mock::given(method("HEAD"))
            .and(path(route))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    for route in ["/missing.msix", "/empty.msix", "/slow.msix"] {
        assert_eq!(
            tokio::time::timeout(
                std::time::Duration::from_millis(3500),
                gateway_host::client_distribution::official_package_size(
                    &client,
                    &format!("{}{route}", server.uri()),
                ),
            )
            .await
            .expect("metadata timeout must not wait for the slow response"),
            None
        );
    }
    server.verify().await;
}
