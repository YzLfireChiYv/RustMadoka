use rustmadoka_core::account::{Channel, Store};
use rustmadoka_core::client::GameClient;
use rustmadoka_core::fingerprint::fetch_fingerprint;
use serde_json::json;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let data = PathBuf::from(r"C:\GrokProject\automadoka\RustMadoka_data");
    let store = Store::open(&data)?;
    let g = store.load_group("123456", Some("123456"))?;
    let acc = g
        .accounts
        .iter()
        .find(|a| a.alias == "群友日服")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no account"))?;
    let ch = Channel::from_user(&acc.channel);
    let fp = fetch_fingerprint(
        "https://raw.githubusercontent.com/YzLfireChiYv/rules/main/automadoka.json",
        ch.as_str(),
    )
    .await
    .or_else(|_| {
        let t = std::fs::read_to_string(data.join("cache/version.json"))?;
        Ok::<_, anyhow::Error>(serde_json::from_str(&t)?)
    })?;
    let mut client =
        GameClient::login(ch.as_str(), &acc.username, &acc.password, fp, &data).await?;
    let stages = client
        .request("/api/mst/get_field_stage_mst_list", json!({}))
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let list = stages
        .get("mstList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let out_dir = data.join("exports/_analysis");
    std::fs::create_dir_all(&out_dir)?;
    let out = out_dir.join("field_stage_mst.json");
    std::fs::write(&out, serde_json::to_string_pretty(&list)?)?;
    println!("wrote {} stages to {}", list.len(), out.display());

    let mut map = std::collections::HashMap::new();
    for s in &list {
        let id = s.get("fieldStageMstId").and_then(|x| x.as_i64()).unwrap_or(0);
        map.insert(id, s);
    }
    let target = 612001i64;
    let mut chain = vec![];
    let mut cur = target;
    let mut guard = 0;
    while cur != 0 && guard < 200 {
        guard += 1;
        if let Some(s) = map.get(&cur) {
            chain.push((
                cur,
                s.get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("?")
                    .to_string(),
                s.get("prevFieldStageMstId")
                    .and_then(|x| x.as_i64())
                    .unwrap_or(0),
                s.get("prevFieldStageMstId2")
                    .and_then(|x| x.as_i64())
                    .unwrap_or(0),
                s.get("difficulty")
                    .and_then(|x| x.as_i64())
                    .unwrap_or(-1),
                s.get("fieldSeriesMstId")
                    .and_then(|x| x.as_i64())
                    .unwrap_or(0),
            ));
            cur = s
                .get("prevFieldStageMstId")
                .and_then(|x| x.as_i64())
                .unwrap_or(0);
        } else {
            println!("missing {cur}");
            break;
        }
    }
    chain.reverse();
    for (i, (id, name, p1, p2, diff, series)) in chain.iter().enumerate() {
        println!(
            "{i:02} id={id} prev={p1}/{p2} diff={diff} series={series} name={name}"
        );
    }

    let col = client
        .request(
            "/api/exploration/get_field_stage_collection_info_list",
            json!({}),
        )
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let cl = col
        .get("fieldStageCollectionInfoList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let cleared: Vec<i64> = cl
        .iter()
        .filter(|x| x.get("isClear").and_then(|b| b.as_bool()).unwrap_or(false))
        .filter_map(|x| x.get("fieldStageMstId").and_then(|i| i.as_i64()))
        .collect();
    println!("cleared_fields count={}", cleared.len());
    println!("612001 cleared? {}", cleared.contains(&612001));
    std::fs::write(
        out_dir.join("field_stage_collection.json"),
        serde_json::to_string_pretty(&cl)?,
    )?;
    Ok(())
}
