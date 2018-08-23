/******************************************************************************
 * Copyright © 2014-2018 The SuperNET Developers.                             *
 *                                                                            *
 * See the AUTHORS, DEVELOPER-AGREEMENT and LICENSE files at                  *
 * the top-level directory of this distribution for the individual copyright  *
 * holder information and the developer policies on copyright and licensing.  *
 *                                                                            *
 * Unless otherwise agreed in a custom licensing agreement, no part of the    *
 * SuperNET software, including this file may be copied, modified, propagated *
 * or distributed except according to the terms contained in the LICENSE file *
 *                                                                            *
 * Removal or modification of this copyright notice is prohibited.            *
 *                                                                            *
 ******************************************************************************/
//
//  mm2.rs
//  marketmaker
//
//  Copyright © 2017-2018 SuperNET. All rights reserved.
//

#![allow(non_camel_case_types)]

extern crate backtrace;

#[allow(unused_imports)]
#[macro_use]
extern crate duct;

#[cfg(feature = "etomic")]
extern crate etomiclibrs;

#[macro_use]
extern crate fomat_macros;

extern crate futures;
extern crate futures_cpupool;

#[macro_use]
extern crate gstuff;

extern crate hyper;

#[allow(unused_imports)]
#[macro_use]
extern crate lazy_static;

extern crate libc;

extern crate nix;

#[macro_use]
extern crate unwrap;

extern crate winapi;

// Re-export preserves the functions that are temporarily accessed from C during the gradual port.
#[cfg(feature = "etomic")]
pub use etomiclibrs::*;

use gstuff::now_ms;

use std::env;
use std::ffi::{CStr, CString, OsString};
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int, c_long, c_void};
use std::mem::{size_of, zeroed};
use std::path::Path;
use std::ptr::{null, null_mut};
use std::str::from_utf8_unchecked;
use std::slice::from_raw_parts;
use std::sync::Mutex;

pub mod crash_reports;
mod lp {include! ("c_headers/LP_include.rs");}
use lp::{cJSON, _bits256 as bits256};
#[allow(dead_code)]
extern "C" {
    fn bitcoin_priv2wif (symbol: *const u8, wiftaddr: u8, wifstr: *mut c_char, privkey: bits256, addrtype: u8) -> i32;
    fn bits256_str (hexstr: *mut u8, x: bits256) -> *const c_char;
    fn LP_main (c_json: *mut cJSON) -> !;
    fn mm1_main (argc: c_int, argv: *const *const c_char) -> !;
}

use crash_reports::init_crash_reports;

impl fmt::Display for bits256 {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf: [u8; 65] = unsafe {zeroed()};
        let cs = unsafe {bits256_str (buf.as_mut_ptr(), *self)};
        let hex = unwrap! (unsafe {CStr::from_ptr (cs)} .to_str());
        f.write_str (hex)
    }
}

/// RAII and MT wrapper for `cJSON`.
#[allow(dead_code)]
struct CJSON (*mut cJSON);
#[allow(dead_code)]
impl CJSON {
    fn from_zero_terminated (json: *const c_char) -> Result<CJSON, String> {
        lazy_static! {static ref LOCK: Mutex<()> = Mutex::new(());}
        let _lock = try_s! (LOCK.lock());  // Probably need a lock to access the error singleton.
        let c_json = unsafe {lp::cJSON_Parse (json)};
        if c_json == null_mut() {
            let err = unsafe {lp::cJSON_GetErrorPtr()};
            let err = try_s! (unsafe {CStr::from_ptr (err)} .to_str());
            ERR! ("Can't parse JSON, error: {}", err)
        } else {
            Ok (CJSON (c_json))
        }
    }
}
impl Drop for CJSON {
    fn drop (&mut self) {
        unsafe {lp::cJSON_Delete (self.0)}
        self.0 = null_mut()
    }
}

#[allow(dead_code,non_upper_case_globals,non_camel_case_types,non_snake_case)]
mod os {include! ("c_headers/OS_portable.rs");}

/*
#endif // !_WIN_32

uint32_t DOCKERFLAG;
#define MAX(a,b) ((a) > (b) ? (a) : (b))
char *stats_JSON(void *ctx,int32_t fastflag,char *myipaddr,int32_t pubsock,cJSON *argjson,char *remoteaddr,uint16_t port);
#include "stats.c"
void LP_priceupdate(char *base,char *rel,double price,double avebid,double aveask,double highbid,double lowask,double PAXPRICES[32]);

*/
#[allow(dead_code,non_upper_case_globals,non_camel_case_types,non_snake_case)]
mod nn {include! ("c_headers/nn.rs");}
/*
#ifndef NN_WS_MSG_TYPE
#define NN_WS_MSG_TYPE 1
#endif


#include "LP_nativeDEX.c"

void LP_ports(uint16_t *pullportp,uint16_t *pubportp,uint16_t *busportp,uint16_t netid)
{
    int32_t netmod,netdiv; uint16_t otherports;
    *pullportp = *pubportp = *busportp = 0;
    if ( netid < 0 )
        netid = 0;
    else if ( netid > (65535-40-LP_RPCPORT)/4 )
    {
        printf("netid.%d overflow vs max netid.%d 14420?\n",netid,(65535-40-LP_RPCPORT)/4);
        exit(-1);
    }
    if ( netid != 0 )
    {
        netmod = (netid % 10);
        netdiv = (netid / 10);
        otherports = (netdiv * 40) + (LP_RPCPORT + netmod);
    } else otherports = LP_RPCPORT;
    *pullportp = otherports + 10;
    *pubportp = otherports + 20;
    *busportp = otherports + 30;
    printf("RPCport.%d remoteport.%d, nanoports %d %d %d\n",RPC_port,RPC_port-1,*pullportp,*pubportp,*busportp);
}

void LP_main(void *ptr)
{
    char *passphrase; double profitmargin; uint16_t netid=0,port,pullport,pubport,busport; cJSON *argjson = ptr;
    if ( (passphrase= jstr(argjson,"passphrase")) != 0 )
    {
        profitmargin = jdouble(argjson,"profitmargin");
        LP_profitratio += profitmargin;
        if ( (port= juint(argjson,"rpcport")) < 1000 )
            port = LP_RPCPORT;
        if ( jobj(argjson,"netid") != 0 )
            netid = juint(argjson,"netid");
        LP_ports(&pullport,&pubport,&busport,netid);
        LPinit(port,pullport,pubport,busport,passphrase,jint(argjson,"client"),jstr(argjson,"userhome"),argjson);
    }
}
*/

fn global_dbdir() -> &'static Path {
    Path::new (unwrap! (unsafe {CStr::from_ptr (lp::GLOBAL_DBDIR.as_ptr())} .to_str()))
}

/// Invokes `OS_ensure_directory`,  
/// then prints an error and returns `false` if the directory is not writeable.
fn ensure_writable (dir_path: &Path) -> bool {
    let c_dir_path = unwrap! (dir_path.to_str());
    let c_dir_path = unwrap! (CString::new (c_dir_path));
    unsafe {os::OS_ensure_directory (c_dir_path.as_ptr() as *mut c_char)};

    /*
    char fname[512],str[65],str2[65]; bits256 r,check; FILE *fp;
    */
    let mut r: [u8; 32] = unsafe {zeroed()};
    let mut check: Vec<u8> = Vec::with_capacity (r.len());
    unsafe {os::OS_randombytes (r.as_mut_ptr(), r.len() as c_long)};
    let fname = dir_path.join ("checkval");
    let mut fp = match fs::File::create (&fname) {
        Ok (fp) => fp,
        Err (_) => {
            eprintln! ("FATAL ERROR cant create {:?}", fname);
            return false
        }
    };
    if fp.write_all (&r) .is_err() {
        eprintln! ("FATAL ERROR writing {:?}", fname);
        return false
    }
    drop (fp);
    let mut fp = match fs::File::open (&fname) {
        Ok (fp) => fp,
        Err (_) => {
            eprintln! ("FATAL ERROR cant open {:?}", fname);
            return false
        }
    };
    if fp.read_to_end (&mut check).is_err() || check.len() != r.len() {
        eprintln! ("FATAL ERROR reading {:?}", fname);
        return false
    }
    if check != r {
        eprintln! ("FATAL ERROR error comparing {:?} {:?} vs {:?}", fname, r, check);
        return false
    }
    true
}

#[cfg(test)]
mod test {
    use duct::Handle;

    use futures::Future;
    use futures_cpupool::CpuPool;

    use gstuff::{now_float, slurp};

    use hyper::{Body, Client, Request, StatusCode};
    use hyper::rt::Stream;

    use std::env;
    use std::fs;
    use std::os::raw::c_char;
    use std::str::{from_utf8, from_utf8_unchecked};
    use std::thread::sleep;
    use std::time::Duration;

    use super::{btc2kmd, events, LP_main, CJSON};

    /// Automatically kill a wrapped process.
    struct RaiiKill {handle: Handle, running: bool}
    impl RaiiKill {
        fn from_handle (handle: Handle) -> RaiiKill {
            RaiiKill {handle, running: true}
        }
        fn running (&mut self) -> bool {
            if !self.running {return false}
            match self.handle.try_wait() {Ok (None) => true, _ => {self.running = false; false}}
        }
    }
    impl Drop for RaiiKill {
        fn drop (&mut self) {
            // The cached `running` check might provide some protection against killing a wrong process under the same PID,
            // especially if the cached `running` check is also used to monitor the status of the process.
            if self.running() {
                let _ = self.handle.kill();
            }
        }
    }

    /// Integration (?) test for the "btc2kmd" command line invocation.
    /// The argument is the WIF example from https://en.bitcoin.it/wiki/Wallet_import_format.
    #[test]
    fn test_btc2kmd() {
        let output = unwrap! (btc2kmd ("5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ"));
        assert_eq! (output, "BTC 5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ \
        -> KMD UpRBUQtkA5WqFnSztd7sCYyyhtd4aq6AggQ9sXFh2fXeSnLHtd3Z: \
        privkey 0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d");
    }

    /// Integration test for the "mm2 events" mode.
    /// Starts MM in background and verifies that "mm2 events" produces a non-empty feed of events.
    #[test]
    fn test_events() {
        let executable = unwrap! (env::args().next());
        let mm_output = env::temp_dir().join ("test_events.mm.log");
        let mm_events_output = env::temp_dir().join ("test_events.mm_events.log");
        match env::var ("MM2_TEST_EVENTS_MODE") {
            Ok (ref mode) if mode == "MM" => {
                println! ("test_events] Starting the MarketMaker...");
                let c_json = unwrap! (CJSON::from_zero_terminated ("{\
                \"gui\":\"nogui\",\
                \"unbuffered-output\":1,\
                \"client\":1,\
                \"passphrase\":\"123\",\
                \"coins\":\"BTC,KMD\"\
                }\0".as_ptr() as *const c_char));
                unsafe {LP_main (c_json.0)}
            },
            Ok (ref mode) if mode == "MM_EVENTS" => {
                println! ("test_events] Starting the `mm2 events`...");
                unwrap! (events (&["_test".into(), "events".into()]));
            },
            _ => {
                // Start the MM.
                println! ("test_events] executable: '{}'.", executable);
                println! ("test_events] `mm2` log: {:?}.", mm_output);
                println! ("test_events] `mm2 events` log: {:?}.", mm_events_output);
                let mut mm = RaiiKill::from_handle (unwrap! (cmd! (&executable, "test_events", "--nocapture")
                    .env ("MM2_TEST_EVENTS_MODE", "MM")
                    .env ("MM2_UNBUFFERED_OUTPUT", "1")
                    .stderr_to_stdout().stdout (&mm_output) .start()));

                let mut mm_events = RaiiKill::from_handle (unwrap! (cmd! (executable, "test_events", "--nocapture")
                    .env ("MM2_TEST_EVENTS_MODE", "MM_EVENTS")
                    .env ("MM2_UNBUFFERED_OUTPUT", "1")
                    .stderr_to_stdout().stdout (&mm_events_output) .start()));

                #[derive(Debug)] enum MmState {Starting, Started, GetendpointSent, Passed}
                let mut mm_state = MmState::Starting;

                // Monitor the MM output.
                let started = now_float();
                loop {
                    if !mm.running() {panic! ("MM process terminated prematurely.")}
                    if !mm_events.running() {panic! ("`mm2 events` terminated prematurely.")}

                    /// Invokes a locally running MM and returns it's reply.
                    fn call_mm (json: String) -> Result<(StatusCode, String), String> {
                        let pool = CpuPool::new (1);
                        let client = Client::builder().executor (pool.clone()) .build_http::<Body>();
                        let fut = pool.spawn (client.request (try_s! (
                            Request::builder().method ("POST") .uri ("http://127.0.0.1:7783") .body (json.into()))));
                        let res = try_s! (fut.wait());
                        let status = res.status();
                        let body = try_s! (pool.spawn (res.into_body().concat2()) .wait());
                        let body = try_s! (from_utf8 (&body)) .trim();
                        Ok ((status, body.into()))
                    }

                    mm_state = match mm_state {
                        MmState::Starting => {  // See if MM started.
                            let mm_log = slurp (&mm_output);
                            let mm_log = unsafe {from_utf8_unchecked (&mm_log)};
                            if mm_log.contains (">>>>>>>>>> DEX stats 127.0.0.1:7783 bind") {MmState::Started}
                            else {MmState::Starting}
                        },
                        MmState::Started => {  // Kickstart the events stream by invoking the "getendpoint".
                            let (status, body) = unwrap! (call_mm (String::from (
                                "{\"userpass\":\"5bfaeae675f043461416861c3558146bf7623526891d890dc96bc5e0e5dbc337\",\"method\":\"getendpoint\"}")));
                            println! ("test_events] getendpoint response: {:?}, {}", status, body);
                            assert_eq! (status, StatusCode::OK);
                            assert! (body.contains ("\"endpoint\":\"ws://127.0.0.1:5555\""));
                            MmState::GetendpointSent
                        },
                        MmState::GetendpointSent => {  // Wait for the `mm2 events` test to finish.
                            let mm_events_log = slurp (&mm_events_output);
                            let mm_events_log = unsafe {from_utf8_unchecked (&mm_events_log)};
                            if mm_events_log.contains ("\"base\":\"KMD\"") && mm_events_log.contains ("\"price64\":\"") {MmState::Passed}
                            else {MmState::GetendpointSent}
                        },
                        MmState::Passed => {  // Gracefully stop the MM.
                            let (status, body) = unwrap! (call_mm (String::from (
                                "{\"userpass\":\"5bfaeae675f043461416861c3558146bf7623526891d890dc96bc5e0e5dbc337\",\"method\":\"stop\"}")));
                            println! ("test_events] stop response: {:?}, {}", status, body);
                            assert_eq! (status, StatusCode::OK);
                            assert_eq! (body, "{\"result\":\"success\"}");
                            sleep (Duration::from_millis (100));
                            let _ = fs::remove_file (mm_output);
                            let _ = fs::remove_file (mm_events_output);
                            break
                        }
                    };

                    if now_float() - started > 20. {panic! ("Test didn't pass withing the 20 seconds timeframe")}
                    sleep (Duration::from_millis (20))
                }
            }
        }
    }
}

fn help() {
    pintln! (
        "Command-line options.\n"
        "The first command-line argument is special and designates the mode.\n"
        "\n"
        "  help                  ..  Display this message.\n"
        "  btc2kmd {WIF or BTC}  ..  Convert a BTC WIF into a KMD WIF.\n"
        "  events                ..  Listen to a feed coming from a separate MM daemon and print it to stdout.\n"
        "  vanity {substring}    ..  Tries to find an address with the given substring.\n"
        "\n"
        // Generated from https://github.com/KomodoPlatform/Documentation (PR to dev branch).
        // SHossain: "this would be the URL we would recommend and it will be maintained
        //            Please let @gcharang or me know if anything needs updating there".
        "See also the online documentation at https://docs.komodoplatform.com/barterDEX/barterDEX-API.html."
    )
}

const MM_VERSION: &'static str = env!("MM_VERSION");

fn main() {
    init_crash_reports();
    unsafe {os::OS_init()};
    println!("BarterDEX MarketMaker {} \n", MM_VERSION);

    // Temporarily simulate `argv[]` for the C version of the main method.
    let args: Vec<String> = env::args().map (|mut arg| {arg.push ('\0'); arg}) .collect();
    let mut args: Vec<*const c_char> = args.iter().map (|s| s.as_ptr() as *const c_char) .collect();
    args.push (null());

    let args_os: Vec<OsString> = env::args_os().collect();

    // NB: The first argument is special, being used as the mode switcher.
    // The other arguments might be used to pass the data to the various MM modes,
    // we're not checking them for the mode switches in order not to risk [untrusted] data being mistaken for a mode switch.
    let first_arg = args_os.get (1) .and_then (|arg| arg.to_str());

    if first_arg == Some ("btc2kmd") && args_os.get (2) .is_some() {
        match btc2kmd (unwrap! (args_os[2].to_str(), "Bad argument encoding")) {
            Ok (output) => println! ("{}", output),
            Err (err) => eprintln! ("btc2kmd error] {}", err)
        }
        return
    }

    if let Err (err) = events (&args_os) {eprintln! ("events error] {}", err); return}

    let second_arg = args_os.get (2) .and_then (|arg| arg.to_str());
    if first_arg == Some ("vanity") && second_arg.is_some() {vanity (unwrap! (second_arg)); return}

    if first_arg == Some ("--help") || first_arg == Some ("-h") || first_arg == Some ("help") {help(); return}
    if cfg! (windows) && first_arg == Some ("/?") {help(); return}

    if !fix_directories() {eprintln! ("Some of the required directories are not accessible."); return}

    unsafe {mm1_main ((args.len() as i32) - 1, args.as_ptr());}
}

// TODO: `btc2kmd` is *pure*, it doesn't use shared state,
// though some of the underlying functions (`LP_convaddress`) do (the hash of cryptocurrencies is shared).
// Should mark it as shallowly pure.

/// Implements the "btc2kmd" command line utility.
fn btc2kmd (wif_or_btc: &str) -> Result<String, String> {
    extern "C" {
        fn LP_wifstr_valid (symbol: *const u8, wifstr: *const u8) -> i32;
        fn LP_convaddress (symbol: *const u8, address: *const u8, dest: *const u8) -> *const c_char;
        fn bitcoin_wif2priv (symbol: *const u8, wiftaddr: u8, addrtypep: *mut u8, privkeyp: *mut bits256, wifstr: *const c_char) -> i32;
        fn bits256_cmp (a: bits256, b: bits256) -> i32;
    }

    let wif_or_btc_z = format! ("{}\0", wif_or_btc);
    /* (this line helps the IDE diff to match the old and new code)
    if ( strstr(argv[0],"btc2kmd") != 0 && argv[1] != 0 )
    */
    let mut privkey: bits256 = unsafe {zeroed()};
    let mut checkkey: bits256 = unsafe {zeroed()};
    let mut tmptype = 0;
    let mut kmdwif: [c_char; 64] = unsafe {zeroed()};
    if unsafe {LP_wifstr_valid (b"BTC\0".as_ptr(), wif_or_btc_z.as_ptr())} > 0 {
        let rc = unsafe {bitcoin_wif2priv (b"BTC\0".as_ptr(), 0, &mut tmptype, &mut privkey, wif_or_btc_z.as_ptr() as *const i8)};
        if rc < 0 {return ERR! ("!bitcoin_wif2priv")}
        let rc = unsafe {bitcoin_priv2wif (b"KMD\0".as_ptr(), 0, kmdwif.as_mut_ptr(), privkey, 188)};
        if rc < 0 {return ERR! ("!bitcoin_priv2wif")}
        let rc = unsafe {bitcoin_wif2priv (b"KMD\0".as_ptr(), 0, &mut tmptype, &mut checkkey, kmdwif.as_ptr())};
        if rc < 0 {return ERR! ("!bitcoin_wif2priv")}
        let kmdwif = try_s! (unsafe {CStr::from_ptr (kmdwif.as_ptr())} .to_str());
        if unsafe {bits256_cmp (privkey, checkkey)} == 0 {
            Ok (format! ("BTC {} -> KMD {}: privkey {}", wif_or_btc, kmdwif, privkey))
        } else {
            Err (format! ("ERROR BTC {} {} != KMD {} {}", wif_or_btc, privkey, kmdwif, checkkey))
        }
    } else {
        let retstr = unsafe {LP_convaddress(b"BTC\0".as_ptr(), wif_or_btc_z.as_ptr(), b"KMD\0".as_ptr())};
        if retstr == null() {return ERR! ("LP_convaddress")}
        Ok (unwrap! (unsafe {CStr::from_ptr (retstr)} .to_str()) .into())
    }
}

/// Implements the `mm2 events` mode.  
/// If the command-line arguments match the events mode and everything else works then this function will never return.
fn events (args_os: &[OsString]) -> Result<(), String> {
    use nn::*;

    /*
    else if ( argv[1] != 0 && strcmp(argv[1],"events") == 0 )
    */
    if args_os.get (1) .and_then (|arg| arg.to_str()) .unwrap_or ("") == "events" {
        let ipc_endpoint = unsafe {nn_socket (AF_SP as c_int, NN_PAIR as c_int)};
        if ipc_endpoint < 0 {return ERR! ("!nn_socket")}
        let rc = unsafe {nn_connect (ipc_endpoint, "ws://127.0.0.1:5555\0".as_ptr() as *const c_char)};
        if rc < 0 {return ERR! ("!nn_connect")}
        loop {
            let mut buf: [u8; 1000000] = unsafe {zeroed()};
            let len = unsafe {nn_recv (ipc_endpoint, buf.as_mut_ptr() as *mut c_void, buf.len() - 1, 0)};
            if len >= 0 {
                let len = len as usize;
                assert! (len < buf.len());
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                try_s! (stdout.write_all (&buf[0..len]));
            }
        }
    }
    Ok(())
}

fn vanity (substring: &str) {
    enum BitcoinCtx {}
    extern "C" {
        fn bitcoin_ctx() -> *mut BitcoinCtx;
        fn bitcoin_priv2pub (
            ctx: *mut BitcoinCtx, symbol: *const u8, pubkey33: *mut u8, coinaddr: *mut u8,
            privkey: bits256, taddr: u8, addrtype: u8);
    }
    /*
    else if ( argv[1] != 0 && strcmp(argv[1],"vanity") == 0 && argv[2] != 0 )
    */
    let mut pubkey33: [u8; 33] = unsafe {zeroed()};
    let mut coinaddr: [u8; 64] = unsafe {zeroed()};
    let mut wifstr: [c_char; 128] = unsafe {zeroed()};
    let mut privkey: bits256 = unsafe {zeroed()};
    let ctx = unsafe {bitcoin_ctx()};
    let timestamp = now_ms() / 1000;
    println! ("start vanitygen ({}).{} t.{}", substring, substring.len(), timestamp);
    for i in 0..1000000000 {
        unsafe {os::OS_randombytes (privkey.bytes.as_mut_ptr(), size_of::<bits256>() as c_long)};
        unsafe {bitcoin_priv2pub (ctx, "KMD\0".as_ptr(), pubkey33.as_mut_ptr(), coinaddr.as_mut_ptr(), privkey, 0, 60)};
        let coinaddr = unsafe {from_utf8_unchecked (from_raw_parts (coinaddr.as_ptr(), 34))};
        // if ( strncmp(coinaddr+1,argv[2],len-1) == 0 )
        if &coinaddr[1 .. substring.len()] == &substring[0 .. substring.len() - 1] {  // Print on near match.
            unsafe {bitcoin_priv2wif ("KMD\0".as_ptr(), 0, wifstr.as_mut_ptr(), privkey, 188)};
            let wifstr = unwrap! (unsafe {CStr::from_ptr (wifstr.as_ptr())} .to_str());
            println! ("i.{} {} -> {} wif.{}", i, privkey, coinaddr, wifstr);
            if coinaddr.as_bytes()[substring.len()] == substring.as_bytes()[substring.len() - 1] {break}  // Stop on full match.
        }
    }
    println! ("done vanitygen.({}) done {} elapsed {}\n", substring, now_ms() / 1000, now_ms() / 1000 - timestamp);
}

fn fix_directories() -> bool {
    unsafe {os::OS_ensure_directory (lp::GLOBAL_DBDIR.as_ptr() as *mut c_char)};
    let dbdir = global_dbdir();
    if !ensure_writable (&dbdir.join ("SWAPS")) {return false}
    if !ensure_writable (&dbdir.join ("GTC")) {return false}
    if !ensure_writable (&dbdir.join ("PRICES")) {return false}
    if !ensure_writable (&dbdir.join ("UNSPENTS")) {return false}
    true
}

/*
    if ( argc == 1 )
    {
        //LP_privkey_tests();
        LP_NXT_redeems();
        sleep(3);
        return(0);
    }
    if ( argc > 1 && (retjson= cJSON_Parse(argv[1])) != 0 )
    {
        if ( jint(retjson,"docker") == 1 )
            DOCKERFLAG = 1;
        else if ( jstr(retjson,"docker") != 0 )
            DOCKERFLAG = (uint32_t)calc_ipbits(jstr(retjson,"docker"));
        //if ( jobj(retjson,"passphrase") != 0 )
        //    jdelete(retjson,"passphrase");
        //if ( (passphrase= jstr(retjson,"passphrase")) == 0 )
        //    jaddstr(retjson,"passphrase","default");
        if ( OS_thread_create(malloc(sizeof(pthread_t)),NULL,(void *)LP_main,(void *)retjson) != 0 )
        {
            printf("error launching LP_main (%s)\n",jprint(retjson,0));
            exit(-1);
        } //else printf("(%s) launched.(%s)\n",argv[1],passphrase);
        incr = 100.;
        while ( LP_STOP_RECEIVED == 0 )
            sleep(100000);
    } else printf("couldnt parse.(%s)\n",argv[1]);

    return 0;
}

*/