use super::{ConnectorOptions, IntifaceCLIErrorEnum, IntifaceError};

use super::frontend::{self, FrontendPBufChannel};
use argh::FromArgs;
use buttplug::device::configuration_manager::{
  DeviceConfigurationManager,
};
use std::fs;
use tracing::Level;
use tokio_util::sync::CancellationToken;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// command line interface for intiface/buttplug.
///
/// Note: Commands are one word to keep compat with C#/JS executables currently.
#[derive(FromArgs)]
struct IntifaceCLIArguments {
  // Options that do something then exit
  /// print version and exit.
  #[argh(switch)]
  version: bool,

  /// print version and exit.
  #[argh(switch)]
  serverversion: bool,

  // Options that set up the server networking
  /// if passed, websocket server listens on all interfaces. Otherwise, only
  /// listen on 127.0.0.1.
  #[argh(switch)]
  wsallinterfaces: bool,

  /// insecure port for websocket servers.
  #[argh(option)]
  wsinsecureport: Option<u16>,

  /// pipe name for ipc server
  #[argh(option)]
  ipcpipe: Option<String>,

  // Options that set up communications with intiface GUI
  /// if passed, output protobufs for parent process via stdio, instead of strings.
  #[argh(switch)]
  frontendpipe: bool,

  // Options that set up Buttplug server parameters
  /// name of server to pass to connecting clients.
  #[argh(option)]
  #[argh(default = "\"Buttplug Server\".to_owned()")]
  servername: String,

  /// path to the device configuration file
  #[argh(option)]
  deviceconfig: Option<String>,

  /// path to user device configuration file
  #[argh(option)]
  userdeviceconfig: Option<String>,

  /// ping timeout maximum for server (in milliseconds)
  #[argh(option)]
  #[argh(default = "0")]
  pingtime: u64,

  /// if passed, server will stay running after client disconnection
  #[argh(switch)]
  stayopen: bool,

  /// set log level for output
  #[allow(dead_code)]
  #[argh(option)]
  log: Option<Level>,

  /// allow raw messages (dangerous, only use for development)
  #[argh(switch)]
  allowraw: bool,  
}

pub fn check_log_level() -> Option<Level> {
  let args: IntifaceCLIArguments = argh::from_env();
  args.log
}

pub fn check_frontend_pipe(token: CancellationToken) -> Option<FrontendPBufChannel> {
  let args: IntifaceCLIArguments = argh::from_env();
  if args.frontendpipe {
    Some(frontend::run_frontend_task(token))
  } else {
    None
  }
}

pub fn parse_options() -> Result<Option<ConnectorOptions>, IntifaceCLIErrorEnum> {
  let args: IntifaceCLIArguments = argh::from_env();

  // Options that will do a thing then exit:
  //
  // - serverversion
  // - generatecert
  if args.serverversion || args.version {
    debug!("Server version command sent, printing and exiting.");
    println!(
      "Intiface CLI (Rust Edition) Version {}, Commit {}, Built {}",
      VERSION,
      env!("VERGEN_SHA_SHORT"),
      env!("VERGEN_BUILD_TIMESTAMP")
    );
    return Ok(None);
  }

  // Options that set up the server networking

  let mut connector_info = ConnectorOptions::default();
  let mut connector_info_set = false;

  if args.wsallinterfaces {
    info!("Intiface CLI Options: Websocket Use All Interfaces option passed.");
    connector_info.ws_listen_on_all_interfaces = true;
    connector_info_set = true;
  }

  if let Some(wsinsecureport) = &args.wsinsecureport {
    info!(
      "Intiface CLI Options: Websocket Insecure Port {}",
      wsinsecureport
    );
    connector_info.ws_insecure_port = Some(*wsinsecureport);
    connector_info_set = true;
  }

  if let Some(ipcpipe) = &args.ipcpipe {
    // TODO We should actually implement pipes :(
    info!("Intiface CLI Options: IPC Pipe Name {}", ipcpipe);
  }

  // If we don't have a device configuration by this point, panic.

  if !connector_info_set {
    return Err(
      IntifaceError::new(
        "Must have a connection argument (wsinsecureport, wssecureport, ipcport) to run!",
      )
      .into(),
    );
  }

  connector_info.server_options.name = args.servername;
  connector_info.server_options.max_ping_time = args.pingtime;
  connector_info.server_options.allow_raw_messages = args.allowraw;

  if args.frontendpipe {
    info!("Intiface CLI Options: Using frontend pipe");
    connector_info.use_frontend_pipe = true;
  }

  if args.stayopen {
    info!("Intiface CLI Options: Leave server open after disconnect.");
    connector_info.stay_open = true;
  }

  // Options that set up Buttplug server parameters

  if let Some(deviceconfig) = &args.deviceconfig {
    info!(
      "Intiface CLI Options: External Device Config {}",
      deviceconfig
    );
    match fs::read_to_string(deviceconfig) {
      Ok(cfg) => connector_info.server_options.device_configuration_json = Some(cfg),
      Err(err) => panic!("Error opening external device configuration: {:?}", err)
    };
  }

  if let Some(userdeviceconfig) = &args.userdeviceconfig {
    info!(
      "Intiface CLI Options: User Device Config {}",
      userdeviceconfig
    );
    match fs::read_to_string(userdeviceconfig) {
      Ok(cfg) => connector_info.server_options.user_device_configuration_json = Some(cfg),
      Err(err) => panic!("Error opening user device configuration: {:?}", err)
    };
  }

  // Make sure we can bring up a DeviceConfigurationManager before handing back
  // to the server to start the listener.

  if let Err(err) = DeviceConfigurationManager::new_with_options(
    connector_info.server_options.allow_raw_messages, 
    &connector_info.server_options.device_configuration_json, 
    &connector_info.server_options.user_device_configuration_json) {
    panic!("Error in creation of Device Configuration Manager: {:?}", err);
  }

  Ok(Some(connector_info))
}
