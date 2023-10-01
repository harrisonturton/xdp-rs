use std::mem::size_of;

/// count packets received over a given network interface
#[derive(argh::FromArgs, Debug)]
struct Args {
    /// network interface name
    #[argh(positional)]
    ifname: String,
    /// which device queue to load the BPF program on
    #[argh(positional)]
    queue_id: u32,
}

/*

1. Create socket
2. xsk_setup_xdp_prog
3. bpf_get_link_xdp_id (can be bpf_xdp_query)
4. bpf_prog_get_fd_by_id
5. xsk_lookup_bpf_maps (https://github.com/digitalocean/linux-coresched/blob/master/tools/lib/bpf/xsk.c#L394)
    - bpf_obj_get_info_by_fd(prog_fd)
    - bpf_obj_get_info_by_fd
    -
6. xsk_set_bpf_maps (https://github.com/digitalocean/linux-coresched/blob/master/tools/lib/bpf/xsk.c#L466)
    - bpf_map_update_elem(xsks_map_fd, queue_id, socket.fd)
*/

pub fn main() -> xdp::Result<()> {
    // Get program info

    let drv_prog_id = get_bpf_drv_prog_id(6, 0)?;
    println!("drv_prog_id: {drv_prog_id}");
    let bpf_fd = get_bpf_fd_by_id(drv_prog_id)?;
    println!("bpf_fd: {bpf_fd}");
    let prog_info = bpf_obj_get_prog_info_by_fd(bpf_fd as i32)?;
    println!("prog_info: {prog_info:?}");

    let mut map_ids = Vec::<u32>::with_capacity(prog_info.nr_map_ids as usize);

    let mut info: libbpf_sys::bpf_prog_info = Default::default();
    info.nr_map_ids = prog_info.nr_map_ids;
    info.map_ids = map_ids.as_mut_ptr().addr() as u64;

    // Do this a second time to get map IDs
    let mut size = size_of::<libbpf_sys::bpf_prog_info>() as u32;
    let ret = unsafe {
        libbpf_sys::bpf_obj_get_info_by_fd(
            bpf_fd as i32,
            &mut info as *mut libbpf_sys::bpf_prog_info as *mut _,
            &mut size as *mut u32,
        )
    };
    assert!(ret >= 0);

    println!("prog info2: {info:?}");

    let map_ids = info.map_ids as *mut u32;
    let map_id = unsafe { map_ids.read() };
    println!("MAP ID 0: {map_id:?}");

    let map_fd = bpf_map_get_fd_by_id(map_id as i32)?;
    println!("MAP FD: {map_fd}");

    let mut info = bpf_obj_get_map_info_by_fd(map_fd)?;
    println!("MAP INFO: {info:?}");

    let map_name = unsafe { CString::from_raw(info.name.as_mut_ptr()) };
    println!("MAP NAME: {map_name:?}");

    // let mut map_id_vec = Vec::<u32>::with_capacity(prog_info.nr_map_ids as usize);
    // let mut size = size_of::<libbpf_sys::bpf_prog_info>() as u32;
    // let mut info = libbpf_sys::bpf_prog_info::default();
    // info.map_ids = map_id_vec.as_mut_ptr().addr() as u64;

    // let ret = unsafe {
    //     libbpf_sys::bpf_obj_get_info_by_fd(
    //         bpf_fd as i32,
    //         &mut info as *mut libbpf_sys::bpf_prog_info as *mut _,
    //         &mut size as *mut u32,
    //     )
    // };
    // if ret < 0 {
    //     println!("ret: {ret}");
    // }

    // println!("{map_id_vec:?}");

    // let name = unsafe { CString::from_raw(prog_info.name.as_mut_ptr() as *mut _) };
    // println!("prog name: {name:?}");

    // info.nr_map_ids and info.map_ids can be used to memset an array of BPF map IDs
    // Then use each map ID to bpf_map_get_fd_by_id
    // And then bpf_obj_get_info_by_fd on the map FD
    // Check info.name
    // If xsks_map, bingo, update elem with socket fd

    Ok(())

    // unsafe {
    //     let mut opts = libbpf_sys::bpf_xdp_query_opts {
    //         prog_id: 1,
    //         sz: size_of::<libbpf_sys::bpf_xdp_query_opts>() as u64,
    //         ..Default::default()
    //     };
    //     let res = libbpf_sys::bpf_xdp_query(5, 0, &mut opts as *mut libbpf_sys::bpf_xdp_query_opts);
    //     println!("error: {}", sys::strerror(-res));
    //     println!("{res:?}");
    //     println!("{opts:?}");

    //     let res = libbpf_sys::bpf_prog_get_fd_by_id(opts.drv_prog_id);
    //     println!("error: {}", sys::strerror(-res));
    //     println!("{res:?}");

    //     let res = libbpf_sys::bpf_map_get_fd_by_id(id);
    // }
}

fn bpf_map_get_fd_by_id(id: i32) -> Result<i32> {
    let ret = unsafe { libbpf_sys::bpf_map_get_fd_by_id(id as u32) };

    if ret < 0 {
        return Err(Error::Bpf(-ret));
    }

    Ok(ret)
}

fn get_bpf_drv_prog_id(ifindex: u32, prog_id: u32) -> Result<u32> {
    let mut opts = libbpf_sys::bpf_xdp_query_opts {
        prog_id,
        sz: size_of::<libbpf_sys::bpf_xdp_query_opts>() as u64,
        ..Default::default()
    };

    let ret = unsafe {
        libbpf_sys::bpf_xdp_query(
            ifindex as i32,
            0,
            &mut opts as *mut libbpf_sys::bpf_xdp_query_opts,
        )
    };

    if ret < 0 {
        return Err(Error::Bpf(-ret));
    }

    Ok(opts.drv_prog_id)
}

fn get_bpf_fd_by_id(drv_prog_id: u32) -> Result<u32> {
    let ret = unsafe { libbpf_sys::bpf_prog_get_fd_by_id(drv_prog_id) };

    if ret < 0 {
        return Err(Error::Bpf(-ret));
    }

    Ok(ret as u32)
}

fn bpf_obj_get_prog_info_by_fd(bpf_fd: i32) -> Result<libbpf_sys::bpf_prog_info> {
    let mut info: libbpf_sys::bpf_prog_info = Default::default();
    let mut size = size_of::<libbpf_sys::bpf_prog_info>() as u32;
    let ret = unsafe {
        libbpf_sys::bpf_obj_get_info_by_fd(
            bpf_fd as i32,
            &mut info as *mut libbpf_sys::bpf_prog_info as *mut _,
            &mut size as *mut u32,
        )
    };

    if ret < 0 {
        return Err(Error::Bpf(-ret));
    }

    Ok(info)
}

fn bpf_obj_get_map_info_by_fd(bpf_fd: i32) -> Result<libbpf_sys::bpf_map_info> {
    let mut info: libbpf_sys::bpf_map_info = Default::default();
    let mut size = size_of::<libbpf_sys::bpf_map_info>() as u32;
    let ret = unsafe {
        libbpf_sys::bpf_obj_get_info_by_fd(
            bpf_fd as i32,
            &mut info as *mut libbpf_sys::bpf_map_info as *mut _,
            &mut size as *mut u32,
        )
    };

    if ret < 0 {
        return Err(Error::Bpf(-ret));
    }

    Ok(info)
}

// pub fn main() -> Result<()> {
//     match cli::exec() {
//         Ok(()) => Ok(()),
//         Err(error::Error::Mmap(code)) => {
//             println!("error: {}", sys::strerror(code));
//             return Ok(());
//         }
//         Err(error::Error::Socket(code)) => {
//             println!("socket error: {}", sys::strerror(code));
//             return Ok(());
//         }
//         Err(error::Error::SetSockOpt(code)) => {
//             println!("setsockopt error: {}", sys::strerror(code));
//             return Ok(());
//         }
//         Err(error::Error::GetSockOpt(code)) => {
//             println!("getsockopt error: {}", sys::strerror(code));
//             return Ok(());
//         }
//         Err(error::Error::Bind(code)) => {
//             println!("bind error: {}", sys::strerror(code));
//             return Ok(());
//         }
//         Err(error::Error::IfNameToIndex(code)) => {
//             println!("if_nametoindex error: {}", sys::strerror(code));
//             return Ok(());
//         }
//         _ => Ok(()),
//     }
// }
