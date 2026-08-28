mod fixtures;
mod utils;

use fixtures::{server, Error, TestServer};
use rstest::rstest;

#[rstest]
fn audit_disabled_by_default(server: TestServer) -> Result<(), Error> {
    let resp = reqwest::blocking::get(format!("{}__dufs__/api/audit", server.url()))?;
    assert_eq!(resp.status(), 403);
    Ok(())
}

#[rstest]
fn audit_enabled_query(#[with(&["--allow-audit"])] server: TestServer) -> Result<(), Error> {
    let resp = reqwest::blocking::get(format!("{}__dufs__/api/audit", server.url()))?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 20);
    Ok(())
}

#[rstest]
fn audit_upload_and_delete_records(
    #[with(&["--allow-all", "--allow-audit"])] server: TestServer,
) -> Result<(), Error> {
    // 1. Upload a file
    let upload_url = format!("{}audit_test.txt", server.url());
    let resp = fetch!(b"PUT", &upload_url)
        .body(b"hello audit".to_vec())
        .send()?;
    assert_eq!(resp.status(), 201);

    // Give a brief moment for async spawn record to finish
    std::thread::sleep(std::time::Duration::from_millis(50));

    // 2. Query audit log
    let audit_url = format!("{}__dufs__/api/audit", server.url());
    let resp = reqwest::blocking::get(&audit_url)?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    assert!(body["total"].as_u64().unwrap_or(0) >= 1);
    let records = body["data"].as_array().unwrap();
    let upload_rec = records
        .iter()
        .find(|r| r["action"] == "UPLOAD" && r["path"] == "audit_test.txt")
        .expect("Upload audit record not found");
    assert_eq!(upload_rec["status"], "SUCCESS");
    assert_eq!(upload_rec["status_code"], 201);

    // 3. Delete the file
    let resp = fetch!(b"DELETE", &upload_url).send()?;
    assert_eq!(resp.status(), 204);

    std::thread::sleep(std::time::Duration::from_millis(50));

    // 4. Query audit log again
    let resp = reqwest::blocking::get(&audit_url)?;
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    let records = body["data"].as_array().unwrap();
    let delete_rec = records
        .iter()
        .find(|r| r["action"] == "DELETE" && r["path"] == "audit_test.txt")
        .expect("Delete audit record not found");
    assert_eq!(delete_rec["status"], "SUCCESS");
    assert_eq!(delete_rec["status_code"], 204);

    // 5. Check stats
    let stats_url = format!("{}__dufs__/api/audit/stats", server.url());
    let resp = reqwest::blocking::get(&stats_url)?;
    assert_eq!(resp.status(), 200);
    let stats: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    assert!(stats["today_total"].as_u64().unwrap_or(0) >= 2);

    // 6. Check CSV export
    let export_url = format!("{}__dufs__/api/audit/export", server.url());
    let resp = reqwest::blocking::get(&export_url)?;
    assert_eq!(resp.status(), 200);
    let csv = resp.text()?;
    assert!(csv.contains("UPLOAD"));
    assert!(csv.contains("DELETE"));
    assert!(csv.contains("audit_test.txt"));

    // 7. Test clear endpoint
    let clear_url = format!("{}__dufs__/api/audit/clear", server.url());
    let resp = fetch!(b"POST", &clear_url).send()?;
    assert_eq!(resp.status(), 200);

    let resp = reqwest::blocking::get(&audit_url)?;
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    assert_eq!(body["total"].as_u64().unwrap_or(1), 0);

    Ok(())
}

#[rstest]
fn audit_index_data_flag(#[with(&["--allow-audit"])] server: TestServer) -> Result<(), Error> {
    let resp = reqwest::blocking::get(format!("{}?json", server.url()))?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    assert_eq!(body["allow_audit"], true);
    Ok(())
}

#[rstest]
fn audit_auth_failure_recorded(
    #[with(&["--allow-audit", "--auth", "admin:admin@/:rw"])] server: TestServer,
) -> Result<(), Error> {
    // Attempt request without auth -> 401
    let url = format!("{}secret.txt", server.url());
    let resp = reqwest::blocking::get(&url)?;
    assert_eq!(resp.status(), 401);

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Authenticated admin checks audit
    let audit_url = format!("{}__dufs__/api/audit", server.url());
    let resp = fetch!(b"GET", &audit_url)
        .header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .send()?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    let records = body["data"].as_array().unwrap();
    let auth_fail = records
        .iter()
        .find(|r| r["action"] == "AUTH_FAIL")
        .expect("AUTH_FAIL record not found");
    assert_eq!(auth_fail["status"], "FAILED");
    assert_eq!(auth_fail["status_code"], 401);

    Ok(())
}

#[rstest]
fn audit_anonymous_forbidden_when_auth_configured(
    #[with(&["--allow-audit", "--auth", "admin:admin@/:rw", "--auth", "@/"])] server: TestServer,
) -> Result<(), Error> {
    // 1. Anonymous GET /?json -> allow_audit is false
    let resp = reqwest::blocking::get(format!("{}?json", server.url()))?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    assert_eq!(body["allow_audit"], false);

    // 2. Anonymous GET /__dufs__/api/audit -> 401 Unauthorized
    let audit_url = format!("{}__dufs__/api/audit", server.url());
    let resp = reqwest::blocking::get(&audit_url)?;
    assert_eq!(resp.status(), 401);

    // 3. Admin GET /?json -> allow_audit is true
    let resp = fetch!(b"GET", format!("{}?json", server.url()))
        .header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .send()?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    assert_eq!(body["allow_audit"], true);

    // 4. Admin GET /__dufs__/api/audit -> 200 OK
    let resp = fetch!(b"GET", &audit_url)
        .header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .send()?;
    assert_eq!(resp.status(), 200);

    Ok(())
}

#[rstest]
fn audit_with_path_prefix_and_subpath_auth(
    #[with(&["--allow-audit", "--path-prefix", "e635d75b-cef4-436d-aff2-ca1a906d2a81", "--auth", "opm:opm@/public:rw", "--auth", "@/"])] server: TestServer,
) -> Result<(), Error> {
    let audit_url = format!("{}e635d75b-cef4-436d-aff2-ca1a906d2a81/__dufs__/api/audit/stats", server.url());
    // 1. Without auth -> 401
    let resp = reqwest::blocking::get(&audit_url)?;
    assert_eq!(resp.status(), 401);

    // 2. With auth -> 200 OK
    let resp = fetch!(b"GET", &audit_url)
        .header("Authorization", "Basic b3BtOm9wbQ==")
        .send()?;
    assert_eq!(resp.status(), 200);

    Ok(())
}

#[rstest]
fn audit_records_x_real_ip(#[with(&["--allow-audit", "--allow-upload"])] server: TestServer) -> Result<(), Error> {
    let upload_url = format!("{}test_ip.txt", server.url());
    let resp = fetch!(b"PUT", &upload_url)
        .header("X-Real-IP", "198.51.100.42")
        .body("hello world")
        .send()?;
    assert_eq!(resp.status(), 201);

    std::thread::sleep(std::time::Duration::from_millis(50));

    let audit_url = format!("{}__dufs__/api/audit", server.url());
    let resp = reqwest::blocking::get(&audit_url)?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = serde_json::from_str(&resp.text()?)?;
    let records = body["data"].as_array().unwrap();
    let record = records
        .iter()
        .find(|r| r["path"] == "test_ip.txt")
        .expect("upload record not found");
    assert_eq!(record["ip"], "198.51.100.42");

    Ok(())
}
