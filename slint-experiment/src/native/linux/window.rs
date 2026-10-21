//! Real compositor window identities and geometry, never synthetic HWNDs.

use super::desktop::{dispatch, query};
use slint::winit_030::WinitWindowAccessor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

type Error = Box<dyn std::error::Error>;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
static DRAG: Mutex<Option<(isize, i32, i32, i32, i32)>> = Mutex::new(None);

fn clients() -> Result<Vec<serde_json::Value>, Error> {
    let value = query("clients")?;
    Ok(value.as_array().ok_or("invalid Hyprland window response")?.iter()
        .filter(|v| v["pid"].as_u64() == Some(u64::from(std::process::id())))
        .cloned().collect())
}
fn client(id: isize) -> Result<serde_json::Value, Error> {
    clients()?.into_iter().find(|v| address(v) == Some(id)).ok_or_else(|| "native Wayland window unavailable".into())
}
fn address(v: &serde_json::Value) -> Option<isize> {
    isize::from_str_radix(v["address"].as_str()?.strip_prefix("0x")?,16).ok()
}
fn selector(id: isize) -> Result<String, Error> { client(id)?; Ok(format!("address:0x{id:x}")) }

pub fn view_id(window: &slint::Window) -> Result<isize, Error> {
    // A unique native title maps the exact Slint window to its compositor address.
    let title = window.with_winit_window(|w| {
        let title = w.title();
        if title.contains(" [suflyor-") { return title; }
        let unique = format!("{title} [suflyor-{}-{}]", std::process::id(), SEQUENCE.fetch_add(1,Ordering::Relaxed));
        w.set_title(&unique); unique
    }).ok_or("Wayland window is not realized")?;
    clients()?.into_iter().find(|c| c["title"].as_str() == Some(&title))
        .and_then(|c| address(&c)).ok_or_else(|| "compositor has not mapped this window yet".into())
}

pub fn rect(id: isize) -> Result<(i32,i32,i32,i32),Error> {
    let c=client(id)?;
    let n=|key: &str,i: usize| -> Result<i32, Error> { c[key][i].as_i64().and_then(|n| i32::try_from(n).ok()).ok_or_else(|| "invalid native window geometry".into()) };
    Ok((n("at",0)?,n("at",1)?,n("size",0)?,n("size",1)?))
}
pub fn move_to(id:isize,x:i32,y:i32)->Result<(),Error>{
    let s=selector(id)?; dispatch("movewindowpixel",&format!("exact {x} {y},{s}"))
}
pub fn resize_move(id:isize,x:i32,y:i32,w:i32,h:i32)->Result<(),Error>{
    let s=selector(id)?;
    if !client(id)?["floating"].as_bool().unwrap_or(false) { dispatch("setfloating",&s)?; }
    dispatch("resizewindowpixel",&format!("exact {} {},{s}",w.max(1),h.max(1)))?;
    move_to(id,x,y)
}
pub fn focus(id:isize)->Result<(),Error>{dispatch("focuswindow",&selector(id)?)}
pub fn foreground(id:isize)->bool { query("activewindow").ok().and_then(|v|address(&v))==Some(id) }
pub fn pin(id:isize,on:bool)->Result<(),Error>{
    let c=client(id)?;
    if c["pinned"].as_bool().unwrap_or(false)!=on {dispatch("pin",&selector(id)?)?;}
    Ok(())
}
pub fn cursor()->Result<(i32,i32),Error>{
    let v=query("cursorpos")?;
    Ok((v["x"].as_f64().ok_or("cursor x unavailable")? as i32,v["y"].as_f64().ok_or("cursor y unavailable")? as i32))
}
pub fn monitors()->Result<Vec<crate::win32::MonitorRect>,Error>{
    let v=query("monitors")?;
    Ok(v.as_array().ok_or("native displays unavailable")?.iter().filter_map(|m|{
        let scale=m["scale"].as_f64()?;
        if scale<=0.0{return None;}
        let (mut w,mut h)=(m["width"].as_f64()?,m["height"].as_f64()?);
        if m["transform"].as_u64().unwrap_or(0)%2==1 {std::mem::swap(&mut w,&mut h);}
        let (x,y)=(m["x"].as_i64()? as i32,m["y"].as_i64()? as i32);
        Some(crate::win32::MonitorRect{left:x,top:y,right:x+(w/scale).round() as i32,bottom:y+(h/scale).round() as i32,is_primary:m["focused"].as_bool().unwrap_or(false)})
    }).collect())
}
pub fn hide_own()->Vec<(isize,bool)>{
    let mut hidden=Vec::new();
    for c in clients().unwrap_or_default(){
        if let Some(id)=address(&c){
            if c["hidden"].as_bool()==Some(true){continue;}
            if dispatch("setprop",&format!("{} alpha 0 override",selector(id).unwrap_or_default())).is_ok(){hidden.push((id,true));}
        }
    }
    hidden
}
pub fn show_own(hidden:&[(isize,bool)]){
    for (id,_) in hidden {if let Ok(s)=selector(*id){let _=dispatch("setprop",&format!("{s} alpha unset"));}}
}
pub fn drag_begin(id:isize){
    if let (Ok((cx,cy)),Ok((x,y,_,_)))=(cursor(),rect(id)){
        *DRAG.lock().unwrap_or_else(std::sync::PoisonError::into_inner)=Some((id,cx,cy,x,y));
    }
}
pub fn drag_update(id:isize){
    let saved=*DRAG.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let (Some((active,cx,cy,x,y)),Ok((nx,ny)))=(saved,cursor()){
        if id==active {let _=move_to(id,x+nx-cx,y+ny-cy);}
    }
}
pub fn drag_end(){*DRAG.lock().unwrap_or_else(std::sync::PoisonError::into_inner)=None;}
