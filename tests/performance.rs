//! 手动性能采样：各后台服务在独立测试进程中运行，避免相互干扰。
#![cfg(windows)]
use std::{ffi::c_void,time::{Duration,Instant}};

#[repr(C)]
#[derive(Default)]
struct Memory {size:u32,faults:u32,peak:usize,working:usize,paged_peak:usize,paged:usize,nonpaged_peak:usize,nonpaged:usize,pagefile:usize,pagefile_peak:usize,private:usize}
#[link(name="kernel32")]
extern "system" {
    fn GetCurrentProcess()->*mut c_void;
    fn GetProcessTimes(process:*mut c_void,created:*mut u64,exited:*mut u64,kernel:*mut u64,user:*mut u64)->i32;
    fn K32GetProcessMemoryInfo(process:*mut c_void,memory:*mut Memory,size:u32)->i32;
}
fn sample()->(f64,usize,usize){unsafe{
    let(mut created,mut exited,mut kernel,mut user)=(0,0,0,0);
    assert_ne!(GetProcessTimes(GetCurrentProcess(),&mut created,&mut exited,&mut kernel,&mut user),0);
    let mut memory=Memory{size:std::mem::size_of::<Memory>()as u32,..Default::default()};
    assert_ne!(K32GetProcessMemoryInfo(GetCurrentProcess(),&mut memory,memory.size),0);
    ((kernel+user)as f64/10_000_000.0,memory.working,memory.private)
}}

#[test]
#[ignore="读取本机系统服务及已配置账号；通过 CAPSULEDOCK_PERF_SERVICE 选择采样模块"]
fn background_service_sample(){
    let service=std::env::var("CAPSULEDOCK_PERF_SERVICE").unwrap_or_else(|_|"idle".into());
    let context=eframe::egui::Context::default();
    let media=capsuledock::smtc::shared();
    let(mut bluetooth,mut system,mut notifications)=(None,None,None);
    match service.as_str(){
        "media"=>{capsuledock::smtc::spawn_poller(media.clone(),context);},
        "bluetooth"=>bluetooth=Some(capsuledock::bt::spawn()),
        "system"=>system=Some(capsuledock::system::observe(context,Default::default(),None)),
        "notifications"=>notifications=Some(capsuledock::system::connect(context,false).expect("已有通知权限")),
        "idle"=>{},_=>panic!("未知采样服务"),
    }
    let before=sample();let started=Instant::now();
    while started.elapsed()<Duration::from_secs(20){
        if let Some(service)=&bluetooth{while service.rx.try_recv().is_ok(){}}
        if let Some(rx)=&system{while rx.try_recv().is_ok(){}}
        if let Some(rx)=&notifications{while rx.try_recv().is_ok(){}}
        std::thread::sleep(Duration::from_millis(100));
    }
    let after=sample();
    println!("service={service} elapsed={:.2}s cpu={:.3}s working={:.2}MiB private={:.2}MiB",started.elapsed().as_secs_f64(),after.0-before.0,after.1 as f64/1048576.0,after.2 as f64/1048576.0);
}
