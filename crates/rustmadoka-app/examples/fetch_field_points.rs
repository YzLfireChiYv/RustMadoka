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
    let acc = g.accounts.iter().find(|a| a.alias == "群友日服").cloned().unwrap();
    let ch = Channel::from_user(&acc.channel);
    let fp = fetch_fingerprint(
        "https://raw.githubusercontent.com/YzLfireChiYv/rules/main/automadoka.json",
        ch.as_str(),
    )
    .await?;
    let mut client = GameClient::login(ch.as_str(), &acc.username, &acc.password, fp, &data).await?;
    let field = 600001i64; // 薔薇園の魔女 前編
    let stratum = client.request("/api/mst/get_field_stratum_mst_list", json!({})).await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let point = client.request("/api/mst/get_field_point_mst_list", json!({})).await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let s_list = stratum.get("mstList").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let p_list = point.get("mstList").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let out_dir = data.join("exports/_analysis");
    std::fs::create_dir_all(&out_dir)?;
    let ss: Vec<_> = s_list.iter().filter(|s| s.get("fieldStageMstId").and_then(|x| x.as_i64())==Some(field)).cloned().collect();
    let mut points = vec![];
    for s in &ss {
        let sid = s.get("fieldStratumMstId").and_then(|x| x.as_i64()).unwrap_or(0);
        for p in &p_list {
            if p.get("fieldStratumMstId").and_then(|x| x.as_i64())==Some(sid) {
                points.push(p.clone());
            }
        }
    }
    // topo sort by prevFieldPointMstId
    let mut by_id = std::collections::HashMap::new();
    for p in &points {
        let id = p.get("fieldPointMstId").and_then(|x| x.as_i64()).unwrap_or(0);
        by_id.insert(id, p);
    }
    let mut roots: Vec<i64> = points.iter()
        .filter_map(|p| {
            let prev = p.get("prevFieldPointMstId").and_then(|x| x.as_i64()).unwrap_or(0);
            let id = p.get("fieldPointMstId").and_then(|x| x.as_i64()).unwrap_or(0);
            if prev == 0 { Some(id) } else { None }
        }).collect();
    roots.sort();
    let mut order = vec![];
    let mut seen = std::collections::HashSet::new();
    fn walk(id: i64, by_id: &std::collections::HashMap<i64, &serde_json::Value>, children: &std::collections::HashMap<i64, Vec<i64>>, order: &mut Vec<i64>, seen: &mut std::collections::HashSet<i64>) {
        if !seen.insert(id) { return; }
        order.push(id);
        if let Some(chs) = children.get(&id) {
            for c in chs { walk(*c, by_id, children, order, seen); }
        }
    }
    let mut children: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for p in &points {
        let id = p.get("fieldPointMstId").and_then(|x| x.as_i64()).unwrap_or(0);
        let prev = p.get("prevFieldPointMstId").and_then(|x| x.as_i64()).unwrap_or(0);
        if prev != 0 {
            children.entry(prev).or_default().push(id);
        }
    }
    for r in roots { walk(r, &by_id, &children, &mut order, &mut seen); }
    // leftover
    for p in &points {
        let id = p.get("fieldPointMstId").and_then(|x| x.as_i64()).unwrap_or(0);
        if !seen.contains(&id) { order.push(id); }
    }
    println!("field={} strata={} points={}", field, ss.len(), points.len());
    for (i, id) in order.iter().enumerate() {
        let p = by_id.get(id).unwrap();
        let name = p.get("name").and_then(|x| x.as_str()).unwrap_or("?");
        let pt = p.get("pointType").and_then(|x| x.as_i64()).unwrap_or(-1);
        let prev = p.get("prevFieldPointMstId").and_then(|x| x.as_i64()).unwrap_or(0);
        let ch = p.get("chapterNum").and_then(|x| x.as_i64()).unwrap_or(0);
        let pv1 = p.get("pointValue1").and_then(|x| x.as_i64()).unwrap_or(0);
        let kind = match pt { 1=>"迷宫dungeon", 2|3|4=>"战斗battle", _=>"other" };
        println!("{i:02} point={id} prev={prev} ch={ch} type={pt}({kind}) val1={pv1} name={name}");
    }
    // save
    std::fs::write(out_dir.join("field_600001_points_ordered.json"), serde_json::to_string_pretty(&order.iter().map(|id| by_id[id]).collect::<Vec<_>>())?)?;
    // also list stages with prev2 != 0 (branch)
    let stages: Vec<_> = serde_json::from_str::<Vec<serde_json::Value>>(&std::fs::read_to_string(out_dir.join("field_stage_mst.json"))?)?;
    let branched: Vec<_> = stages.iter().filter(|s| s.get("prevFieldStageMstId2").and_then(|x| x.as_i64()).unwrap_or(0)!=0).collect();
    println!("stages_with_prev2={}", branched.len());
    for s in branched.iter().take(15) {
        println!("branch id={} prev={}/{} name={}", 
            s.get("fieldStageMstId").and_then(|x| x.as_i64()).unwrap_or(0),
            s.get("prevFieldStageMstId").and_then(|x| x.as_i64()).unwrap_or(0),
            s.get("prevFieldStageMstId2").and_then(|x| x.as_i64()).unwrap_or(0),
            s.get("name").and_then(|x| x.as_str()).unwrap_or("?"));
    }
    Ok(())
}
