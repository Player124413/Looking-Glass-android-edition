//! Continuous Clockwork input course. Diagnostic surveys never alter the world.
use super::*;
use crate::{movement::Controls, route::Route};
fn owner(r: &Route) -> &Clockwork { r.interactions.levels.iter().find_map(|s| s.ctl.downcast_ref()).unwrap() }
pub(super) fn check(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut r = if let Ok(f) = std::env::var("LOOKING_GLASS_CLOCKWORK_FROM") {
            Route::resume(a, &serde_json::from_slice(&std::fs::read(f)?)?)?
        } else { let mut r=Route::new(a,"hatter1",Some("hatter1_start1"))?; r.enable_native_cast(a)?; r };
        let result = drive(&mut r);
        crate::campaign_playtest::diagnostic_save(a, &r, "private/hatter1-route/view")?;
        result?;
        let next=r.depart(a,true)?;
        ensure!(next.level().map=="hatter2" && next.world.body_clear(next.player.feet), "Blocked About Face arrival");
        Ok(())
    })
}
pub(crate) fn drive(r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/hatter1-route")?;
    r.tactics=true; r.stop_at_exit=true;
    r.ice_stream=true;
    r.conserve_will=false;
    let text=std::env::var("LOOKING_GLASS_CLOCKWORK_INPUT").ok().map(std::fs::read_to_string).transpose()?.unwrap_or_else(||include_str!("route_steps.json").into());
    let steps: Vec<serde_json::Value>=serde_json::from_str(&text)?;
    for (n,s) in steps.iter().enumerate() {
        println!("CLOCKWORK {n} {s} at {:?}, sanity {}",r.player.feet,r.stats.sanity());
        let result=(|| -> Result<()> {
            if let Some(v)=s.get("nav") {r.navigate(Vec3::from_array(serde_json::from_value(v.clone())?))?;}
            else if let Some(v)=s.get("walk") {r.walk(Vec3::from_array(serde_json::from_value(v.clone())?),false)?;}
            else if let Some(v)=s.get("input") {
                let p:[f32;6]=serde_json::from_value(v.clone())?;
                for k in 0..(p[0]*120.) as usize {r.tick(Controls{wish:vec2(p[1],p[2]),swim:vec3(p[1],p[2],p[3]),rise:p[3],jump:p[4]>0.&&k==0,use_pressed:p[5]>0.&&k==0,run:true,..Default::default()})?;}
            } else if let Some(v)=s.get("portal") {
                let goal=Vec3::from_array(serde_json::from_value(v.clone())?);
                let before=r.teleports;
                for n in 0..1800 {
                    r.tick(Controls {wish:(goal-r.player.feet).truncate().normalize_or_zero(),
                        jump:(r.player.grounded && n%40==0)||r.player.ledge.as_ref().is_some_and(|h|!h.pulling),
                        run:true,..Default::default()})?;
                    if r.teleports>before {break;}
                }
                ensure!(r.teleports==before+1,"Portal not entered at {:?}",r.player.feet);
            } else if let Some(v)=s.get("lever") {
                let k=v.as_u64().context("Invalid lever")? as usize;
                let target=owner(r).levers.get(k).context("Unknown lever")?.translation+Vec3::Z*20.;
                let aim=(target-r.player.eye()).normalize_or_zero();
                ensure!(owner(r).use_lever(&r.world,r.player.eye(),aim)==Some(k),"Lever {k} unavailable at {:?}",r.player.feet);
                r.tick(Controls{wish:aim.truncate().normalize_or_zero(),use_pressed:true,..Default::default()})?;
                for _ in 0..120*7 {
                    if owner(r).saved.pull.is_none() { break; }
                    r.tick(Controls::default())?;
                }
                ensure!(owner(r).saved.levers[k].is_some(),"Lever not activated");
            } else if let Some(v)=s.get("node") {
                let id=v.as_u64().context("Invalid pathnode")? as usize;
                let e=r.map.entities.get(id).context("Unknown pathnode")?;
                ensure!(e.get("classname").is_some_and(|c|c=="info_pathnode"),"Not a pathnode");
                let raw=at(e).translation;
                let floor=r.world.actor_footing(raw+Vec3::Z*32.,PLAYER_CENTER,PLAYER_HALF,240.).context("Unsupported pathnode")?;
                r.navigate(floor)?;
            } else if let Some(v)=s.get("wait") { r.wait(v.as_f64().context("Invalid wait")? as f32)?; }
            else if let Some(v)=s.get("clear") {r.clear(v.as_f64().context("Invalid range")? as f32)?;}
            else if let Some(v)=s.get("survey") { survey(r,serde_json::from_value(v.clone())?)?;anyhow::bail!("Floor survey only; no traversal pass");}
            else {anyhow::bail!("Unknown Clockwork input {s}");}
            Ok(())
        })();
        std::fs::write("private/hatter1-route/last.json",serde_json::to_vec(&r.checkpoint())?)?;
        result?;r.wait_for_cinematic()?;
        if r.transition.is_some(){break;}
    }
    ensure!(r.transition.as_ref().is_some_and(|e|e.0=="hatter2"),"Clockwork course incomplete at {:?}",r.player.feet);
    let s=&owner(r).saved;
    ensure!(s.levers.iter().all(Option::is_some)&&s.hare.is_some()&&s.port.is_some()&&s.gryphon.is_some()&&s.cubes==[true;4]&&s.stopped,"Missing Clockwork progression");
    ensure!(r.stats.alive()&&!r.stats.god&&!r.stats.notarget&&r.audit.lost_ticks==0,"Invalid Clockwork route");
    ensure!(r.teleports==2,"Clockwork used an unintended recovery teleport");
    Ok(())
}
fn survey(r:&Route,b:[i32;6])->Result<()> {
    let mut points=Vec::new();
    for x in (b[0]..=b[1]).step_by(32) {for y in (b[2]..=b[3]).step_by(32) {
        for z in (b[4]..=b[5]).step_by(32) {
            let high=vec3(x as f32,y as f32,z as f32);let low=high-Vec3::Z*32.;
            if !r.world.body_clear(high){continue;}
            let tr=r.world.body_trace(high,low);
            if !tr.start_solid&&tr.fraction<1.&&tr.normal.z>=0.65 {points.push(high.lerp(low,tr.fraction).to_array());}
        }
    }}
    std::fs::write("private/hatter1-route/floors.json",serde_json::to_vec(&points)?)?;
    println!("{} supported floor points",points.len());Ok(())
}
