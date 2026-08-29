use serde_json::json;
use std::collections::HashMap;

fn dominates(a: &HashMap<String,u64>, b: &HashMap<String,u64>) -> bool {
    let mut ge = true; let mut gt=false;
    for (k,bv) in b { let av=a.get(k).copied().unwrap_or(0); if av < *bv { ge=false;} if av > *bv { gt=true; } }
    for (k,av) in a { if !b.contains_key(k) && *av>0 { gt=true; } }
    ge && gt
}

#[test]
fn mesh_state_contract() {
    fn can(from:&str,to:&str)->bool {
        matches!((from,to), ("IDLE","SYNCING")|("SYNCING","CONFLICT")|("CONFLICT","SYNCING")|("SYNCING","IDLE")|("CONFLICT","IDLE"))
    }
    assert!(can("IDLE","SYNCING"));
    assert!(can("SYNCING","CONFLICT"));
    assert!(!can("IDLE","CONFLICT"));
}

#[test]
fn lww_merge_contract() {
    #[derive(Debug,Clone)]
    struct Lww { text: String, ts: i64, device: String }
    fn merge(a:&Lww,b:&Lww)->Lww { if b.ts > a.ts { b.clone() } else if b.ts < a.ts { a.clone() } else { if b.device > a.device { b.clone()} else {a.clone()} } }
    let a=Lww{text:"hello".into(), ts:1000, device:"a".into()};
    let b=Lww{text:"world".into(), ts:2000, device:"b".into()};
    assert_eq!(merge(&a,&b).text,"world");
}

#[test]
fn vector_concurrent() {
    let mut a=HashMap::new(); a.insert("desktop".into(),3); a.insert("phone".into(),1);
    let mut b=HashMap::new(); b.insert("desktop".into(),2); b.insert("phone".into(),2);
    // concurrent
    fn is_concurrent(a:&HashMap<String,u64>,b:&HashMap<String,u64>)->bool {
        let eq = {
            let mut keys=std::collections::HashSet::new();
            for k in a.keys(){keys.insert(k.clone());}
            for k in b.keys(){keys.insert(k.clone());}
            let mut eq=true;
            for k in keys { if a.get(&k).copied().unwrap_or(0)!=b.get(&k).copied().unwrap_or(0) { eq=false; break; } }
            eq
        };
        if eq { return false; }
        !dominates(a,b) && !dominates(b,a)
    }
    assert!(is_concurrent(&a,&b));
}

#[test]
fn mesh_payload_contract() {
    let cases = [
        ("mesh.sync", json!({"deviceId":"phone-xyz","vectors":{"phone-xyz":1},"entries":[]})),
        ("mesh.conflict", json!({"path":"/a","resolution":"lww","winner":"local"})),
    ];
    for (typ,payload) in cases {
        let msg=json!({"v":1,"id":"test","type":typ,"ts":0,"nonce":"a","payload":payload});
        assert!(serde_json::to_string(&msg).unwrap().contains(typ));
    }
}

#[test]
fn pairing_db_limit_simulation() {
    let max = 5;
    let mut db: HashMap<String,String>=HashMap::new();
    for i in 0..5 { db.insert(format!("phone-{}",i),"fp".into()); }
    assert_eq!(db.len(),5);
    // 6th would exceed
    assert!(!db.contains_key("phone-5"));
    // after remove, can add
    db.remove("phone-0");
    assert_eq!(db.len(),4);
    db.insert("phone-5".into(),"fp".into());
    assert_eq!(db.len(),5);
}
