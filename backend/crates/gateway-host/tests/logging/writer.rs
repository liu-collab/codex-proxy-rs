//! 验证日志压缩、轮转，以及重启后的自然日与小数天数保留窗口

use super::*;
use std::time::SystemTime;

const ROTATION_RECORDS: usize = 3;

#[test]
fn retention_preserves_complete_dates_across_compression_rotation_and_restart() {
    if env::var_os(CHILD_PROCESS_ENV).is_some() {
        with_file_logging(
            PathBuf::from(env::var_os(LOG_DIRECTORY_ENV).unwrap()),
            false,
            || {
                let payload = "x".repeat(1024 * 1024);
                // 三条记录已跨过 1 MiB 轮转边界；保留日期的 25 个分段由下方单独构造
                for sequence in 0..ROTATION_RECORDS {
                    tracing::info!(target: APPLICATION_LOG_TARGET, sequence, payload, "retention record");
                    tracing::info!(target: OAUTH_RECOVERY_LOG_TARGET, sequence, payload, "retention record");
                }
            },
        );
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let today = gateway_core::time::DeploymentTimeZone::default()
        .local(chrono::Utc::now())
        .date_naive();
    let mut retained = Vec::new();
    let mut expired = Vec::new();
    for (prefix, days) in [
        (APPLICATION_LOG_FILE_PREFIX, 7),
        (OAUTH_RECOVERY_LOG_FILE_PREFIX, 7),
    ] {
        let boundary = today - chrono::Days::new(days);
        for segment in 1..=25 {
            let path = directory
                .path()
                .join(format!("{prefix}{boundary}.{segment}.log"));
            seed_log(&path, boundary, &format!("boundary-{segment}\n"));
            retained.push(path);
        }
        let old_date = boundary - chrono::Days::new(1);
        let path = directory.path().join(format!("{prefix}{old_date}.log"));
        seed_log(&path, old_date, "expired date\n");
        expired.push(path);
        // 时区切换或恢复的文件按较近写入日期保护整个分段组
        let restored_date = old_date - chrono::Days::new(1);
        for segment in 0..=1 {
            let path = directory
                .path()
                .join(format!("{prefix}{restored_date}.{segment}.log"));
            seed_log(
                &path,
                if segment == 0 { today } else { restored_date },
                "restored date\n",
            );
            retained.push(path);
        }
    }
    let unrelated = directory.path().join("unmanaged.log");
    fs::write(&unrelated, "unmanaged\n").unwrap();
    // 归档发布前崩溃会留下原文件，也可能留下未完成的临时文件
    fs::write(
        retained[0].with_extension("log.gz.tmp"),
        "unfinished archive",
    )
    .unwrap();
    for run in 1..=2 {
        let output = Command::new(env::current_exe().unwrap())
            .args(["--exact", "logging::writer::retention_preserves_complete_dates_across_compression_rotation_and_restart"])
            .env(CHILD_PROCESS_ENV, "1").env(LOG_DIRECTORY_ENV, directory.path())
            .env("RUST_LOG", "off,logging_test_application=info").output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            retained
                .iter()
                .all(|path| path.with_extension("log.gz").exists())
        );
        assert!(
            expired
                .iter()
                .all(|path| !path.exists() && !path.with_extension("log.gz").exists())
        );
        assert!(unrelated.exists());

        for prefix in [APPLICATION_LOG_FILE_PREFIX, OAUTH_RECOVERY_LOG_FILE_PREFIX] {
            let body = read_log_file_set(directory.path(), prefix);
            let mut sequences = [0; ROTATION_RECORDS];
            let mut boundaries = std::collections::BTreeSet::new();
            for line in body.lines() {
                if let Some(segment) = line.strip_prefix("boundary-") {
                    boundaries.insert(segment.parse::<usize>().unwrap());
                } else if line.starts_with('{') {
                    #[derive(serde::Deserialize)]
                    struct Record {
                        fields: Fields,
                    }
                    #[derive(serde::Deserialize)]
                    struct Fields {
                        sequence: Option<usize>,
                    }
                    if let Some(sequence) = serde_json::from_str::<Record>(line)
                        .unwrap()
                        .fields
                        .sequence
                    {
                        sequences[sequence] += 1;
                    }
                }
            }
            assert_eq!(sequences, [run; ROTATION_RECORDS]);
            assert_eq!(
                boundaries,
                (1..=25).collect(),
                "whole boundary date must survive"
            );
            let today_segments = fs::read_dir(directory.path())
                .unwrap()
                .filter(|entry| {
                    entry
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with(&format!("{prefix}{today}"))
                })
                .count();
            assert!(today_segments >= 2, "records must exercise size rotation");
        }
    }
}

#[test]
fn request_dump_retention_uses_elapsed_time_across_compression_and_restart() {
    const RETENTION_ENV: &str = "CPR_LOGGING_TEST_RETENTION_DAYS";
    if env::var_os(CHILD_PROCESS_ENV).is_some() {
        let mut config =
            logging_config(PathBuf::from(env::var_os(LOG_DIRECTORY_ENV).unwrap()), true);
        config.logging.request_dump_retention_days =
            env::var(RETENTION_ENV).unwrap().parse().unwrap();
        with_logging(config, || {
            let payload = "x".repeat(1024 * 1024);
            for sequence in 0..ROTATION_RECORDS {
                tracing::info!(target: REQUEST_DUMP_LOG_TARGET, sequence, payload, "elapsed retention record");
            }
        });
        return;
    }

    for days in [0.25, 0.5, 1.0, 1.5] {
        let directory = tempfile::tempdir().unwrap();
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::seconds((days * 86_400.0) as i64);
        let mut retained = Vec::new();
        let mut expired = Vec::new();
        // 同一日期内既有过期文件也有有效文件，文件名日期不覆盖实际修改时间
        for (segment, modified, keep) in [
            (0, cutoff - chrono::Duration::minutes(1), false),
            (1, cutoff + chrono::Duration::minutes(1), true),
            (2, now + chrono::Duration::hours(1), true),
        ] {
            let path = directory.path().join(format!(
                "{REQUEST_DUMP_LOG_FILE_PREFIX}{}.{segment}.log",
                now.date_naive()
            ));
            fs::write(&path, format!("seed-{segment}\n")).unwrap();
            fs::File::open(&path)
                .unwrap()
                .set_modified(modified.into())
                .unwrap();
            if keep {
                let modified = path.metadata().unwrap().modified().unwrap();
                retained.push((path, modified));
            } else {
                expired.push(path);
            }
        }
        // 归档的修改时间也参与清理，压缩不能重置保留窗口
        let archive = directory.path().join(format!(
            "{REQUEST_DUMP_LOG_FILE_PREFIX}{}.3.log.gz",
            now.date_naive() - chrono::Days::new(3)
        ));
        let mut encoder = flate2::write::GzEncoder::new(
            fs::File::create(&archive).unwrap(),
            flate2::Compression::fast(),
        );
        std::io::Write::write_all(&mut encoder, b"expired archive\n").unwrap();
        encoder
            .finish()
            .unwrap()
            .set_modified((cutoff - chrono::Duration::hours(1)).into())
            .unwrap();
        expired.push(archive);
        let unrelated = directory.path().join("unmanaged.log");
        fs::write(&unrelated, "unmanaged\n").unwrap();

        for run in 1..=2 {
            let output = Command::new(env::current_exe().unwrap())
                .args(["--exact", "logging::writer::request_dump_retention_uses_elapsed_time_across_compression_and_restart"])
                .env(CHILD_PROCESS_ENV, "1")
                .env(LOG_DIRECTORY_ENV, directory.path())
                .env(LOG_TIMEZONE_ENV, "America/New_York")
                .env(RETENTION_ENV, days.to_string())
                .env("RUST_LOG", "off")
                .output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            for (path, modified) in &retained {
                let archive = path.with_extension("log.gz");
                assert!(archive.exists());
                assert_eq!(archive.metadata().unwrap().modified().unwrap(), *modified);
            }
            assert!(
                expired
                    .iter()
                    .all(|path| !path.exists() && !path.with_extension("log.gz").exists())
            );
            assert!(unrelated.exists());
            let body = read_log_file_set(directory.path(), REQUEST_DUMP_LOG_FILE_PREFIX);
            assert!(body.contains("seed-1\n") && body.contains("seed-2\n"));
            assert_eq!(
                body.matches("elapsed retention record").count(),
                run * ROTATION_RECORDS
            );
        }
    }
}

#[test]
fn request_dump_rotates_small_files_when_the_time_window_elapses() {
    if env::var_os(CHILD_PROCESS_ENV).is_some() {
        let directory = PathBuf::from(env::var_os(LOG_DIRECTORY_ENV).unwrap());
        let mut config = logging_config(directory.clone(), true);
        config.logging.request_dump_retention_days = 1.0 / 86_400.0;
        with_logging(config, || {
            tracing::info!(target: REQUEST_DUMP_LOG_TARGET, "before retention window");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let body = read_log_file_set(&directory, REQUEST_DUMP_LOG_FILE_PREFIX);
                if body.contains("before retention window") {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "first record was not flushed"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            std::thread::sleep(std::time::Duration::from_millis(1100));
            tracing::info!(target: REQUEST_DUMP_LOG_TARGET, "after retention window");
        });
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env::current_exe().unwrap())
        .args([
            "--exact",
            "logging::writer::request_dump_rotates_small_files_when_the_time_window_elapses",
        ])
        .env(CHILD_PROCESS_ENV, "1")
        .env(LOG_DIRECTORY_ENV, directory.path())
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = read_log_file_set(directory.path(), REQUEST_DUMP_LOG_FILE_PREFIX);
    assert!(!body.contains("before retention window"));
    assert!(body.contains("after retention window"));
}

fn seed_log(path: &Path, date: chrono::NaiveDate, body: &str) {
    fs::write(path, body).unwrap();
    let modified = gateway_core::time::DeploymentTimeZone::default()
        .date_start(date)
        .unwrap();
    fs::File::open(path)
        .unwrap()
        .set_modified(SystemTime::from(modified))
        .unwrap();
}
