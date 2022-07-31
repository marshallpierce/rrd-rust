use bitflags::bitflags;
use data::Data;
use std::{
    ffi::{CStr, CString},
    path::Path,
    ptr::null,
    time::{Duration, SystemTime},
};

use crate::{
    error::{RrdError, RrdResult},
    util::{path_to_str, ArrayOfStrings, NullTerminatedArrayOfStrings},
};

pub mod data;
pub mod error;
mod sys;
pub mod util;

pub fn create(
    filename: &Path,
    pdp_step: Duration,
    last_up: SystemTime,
    no_overwrite: bool,
    sources: &[&Path],
    template: Option<&Path>,
    args: &[&str],
) -> RrdResult<()> {
    let filename = CString::new(path_to_str(filename)?)?;
    let sources = sources
        .iter()
        .map(|p| path_to_str(p))
        .collect::<Result<Vec<_>, _>>()?;
    let sources = NullTerminatedArrayOfStrings::new(sources)?;
    let template = match template {
        None => None,
        Some(p) => Some(CString::new(path_to_str(p)?)?),
    };
    let args = ArrayOfStrings::new(args)?;

    let rc = unsafe {
        sys::rrd_create_r2(
            filename.as_ptr(),
            pdp_step.as_secs() as sys::c_ulong,
            util::to_unix_time(last_up).unwrap(),
            if no_overwrite { 1 } else { 0 },
            sources.as_ptr(),
            template.map_or(null(), |s| s.as_ptr()),
            args.len() as sys::c_int,
            args.as_ptr(),
        )
    };
    match rc {
        0 => Ok(()),
        _ => Err(RrdError::LibRrdError(get_error())),
    }
}

pub fn fetch(
    filename: &Path,
    cf: &str,
    start: SystemTime,
    end: SystemTime,
    step: Duration,
) -> RrdResult<Data> {
    // in
    let filename = CString::new(path_to_str(filename)?)?;
    let cf = CString::new(cf)?;

    let mut data = Data {
        // in/out
        start: util::to_unix_time(start).unwrap() as sys::c_time_t,
        end: util::to_unix_time(end).unwrap() as sys::c_time_t,
        step: step.as_secs() as sys::c_ulong,

        // out
        ..Default::default()
    };

    let rc = unsafe {
        sys::rrd_fetch_r(
            filename.as_ptr(),
            cf.as_ptr(),
            &mut data.start,
            &mut data.end,
            &mut data.step,
            &mut data.ds_count,
            &mut data.ds_names,
            &mut data.data,
        )
    };
    match rc {
        0 => Ok(data),
        _ => Err(RrdError::LibRrdError(get_error())),
    }
}

bitflags! {
    pub struct ExtraFlags : sys::c_int {
        const SKIP_PAST_UPDATES = 0x01;
    }
}

pub fn update(
    filename: &Path,
    template: Option<&Path>,
    extra_flags: ExtraFlags,
    args: &[&str],
) -> RrdResult<()> {
    let filename = CString::new(path_to_str(filename)?)?;
    let template = match template {
        None => None,
        Some(p) => Some(CString::new(path_to_str(p)?)?),
    };
    let args = ArrayOfStrings::new(args)?;
    let rc = unsafe {
        sys::rrd_updatex_r(
            filename.as_ptr(),
            template.map_or(null(), |s| s.as_ptr()),
            extra_flags.bits(),
            args.len() as sys::c_int,
            args.as_ptr(),
        )
    };
    match rc {
        0 => Ok(()),
        _ => Err(RrdError::LibRrdError(get_error())),
    }
}

fn get_error() -> String {
    unsafe {
        let p = sys::rrd_get_error();
        let s = CStr::from_ptr(p);
        s.to_str().unwrap().to_owned()
    }
}
