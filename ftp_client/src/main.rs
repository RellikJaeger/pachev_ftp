extern crate argparse; //argument parsing such as -h -d etc..
extern crate rpassword; //hidden passwords
extern crate ini;

//Reading from config files
use ini::Ini;


use std::io::prelude::*; //the standard io functions that come with rust
use std::process;
use std::io::BufReader; //the standard io functions that come with rust
use std::net::TcpStream;
use std::net::SocketAddrV4;
use std::thread::spawn; //For threads
use std::io;

use std::string::String;
use std::str::FromStr;
use std::net::ToSocketAddrs;

use std::env; //To collect arguements and variables
use std::process::exit; //Gracefully exiting
use std::iter::Iterator;
use std::collections::HashMap;

use argparse::{ArgumentParser, Print, Store, StoreOption, StoreTrue, StoreFalse};
use rpassword::read_password;
use rpassword::prompt_password_stdout;

//helper files for client functions
mod client;
mod utils;



//This section here defines the arguements that the ftp_client will
//initally take when being called
#[derive(Debug, Clone)]
struct Arguements {
    hostname: String,
    ftp_port: String,
    ftp_mode: String,
    username: Option<String>,
    password: Option<String>,
    passive: bool,
    debug: bool,
    verbose: bool,
    data_port_range: String,
    run_test_file: String,
    config_file: String,
    run_default: bool,
    l_all: String,
    l_only: String,
    log_file: String,
}

//These are the defaults incase no arguements are provided
impl Arguements {
    fn new() -> Arguements {
        Arguements {
            hostname: "".to_string(),
            ftp_port: "21".to_string(),
            ftp_mode: "PASSIVE".to_string(),
            username: None,
            password: None,
            passive: true,
            debug: false,
            verbose: false,
            data_port_range: "".to_string(),
            run_test_file: "".to_string(),
            config_file: "".to_string(),
            run_default: false,
            l_all: "".to_string(),
            l_only: "logs/ftpclient.log".to_string(),
            log_file: "".to_string(),
        }
    }
}


fn main() {

    //Using argparse to make cmd line parsing manageable
    let mut arguements = Arguements::new();
    let conf = Ini::load_from_file("fclient.cfg").unwrap();

    //Loading default setting from conf file
    load_defaults(&mut arguements, &conf);

    //This is due to borrowing issue I'm setting a default mode of true
    //but use the argparser to allow the user to set the transfer mode
    let mut passive = true;

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Pachev's FTP client");

        ap.refer(&mut arguements.hostname)
            .add_argument("hostname", Store, "Server hostname");

        ap.refer(&mut arguements.ftp_port)
            .add_argument("port", Store, "Server Port");

        ap.add_option(&["--info", "-i", "--list-commands"],
                      Print(utils::COMMANDS_HELP.to_string()),
                      "List supported commands");
        ap.add_option(&["--version", "-v"],
                      Print("v0.1.0".to_string()),
                      "Prints version");

        ap.refer(&mut arguements.username)
            .add_option(&["-u", "--user"], StoreOption, "Username");

        ap.refer(&mut arguements.password)
            .add_option(&["-w", "--pass"], StoreOption, "Password");

        ap.refer(&mut arguements.passive)
            .add_option(&["--pasive"],
                        StoreTrue,
                        "Use passive mode and 
                                listen on \
                         provided address for data transfers");
        ap.refer(&mut passive)
            .add_option(&["--active"],
                        StoreFalse,
                        "Use active mode and 
                                listen on provided \
                         address for data transfers");

        ap.refer(&mut arguements.debug)
            .add_option(&["-D", "--debug"], StoreTrue, "Sets debug mode on");

        ap.refer(&mut arguements.verbose)
            .add_option(&["-V", "--verbose"], StoreTrue, "Sets verbose  mode on");

        ap.refer(&mut arguements.data_port_range)
            .add_option(&["--dpr"], Store, "Sets a range of ports for data");

        ap.refer(&mut arguements.config_file)
            .add_option(&["-c", "--config"], Store, "location of configuration file");

        ap.refer(&mut arguements.run_test_file)
            .add_option(&["-t", "--test-file"], Store, "location of test file");

        ap.refer(&mut arguements.run_default)
            .add_option(&["-T"], StoreTrue, "Runs default test file");

        ap.refer(&mut arguements.l_all)
            .add_option(&["--LALL"], Store, "Location to store all log output");

        ap.refer(&mut arguements.l_only)
            .add_option(&["--LONLY"], Store, "Location to store all log output");

        ap.parse_args_or_exit();
    }
    arguements.passive = passive;

    //Uses either the parsed info or defaults to determiner server

    start_ftp_client(&mut arguements);
}

fn start_ftp_client(mut arguements: &mut Arguements) -> BufReader<TcpStream> {

    //this will serve as a holder
    let mut myclient: TcpStream;

    /*
     * Here is the loop for starting the program
     * this handles the cases where no localhost is provided and
     * where an empty hostname is provided. The loop will continue
     * as long as we are not able to connec to a socket. The only command
     * available during the loop are open, quit, help
     */
    loop {


        if !arguements.hostname.is_empty() {
            let server = format!("{}:{}", arguements.hostname, arguements.ftp_port);
            match TcpStream::connect(server.as_str()) {
                Ok(stream) => {
                    arguements.hostname = "".to_string();
                    arguements.ftp_port = "".to_string();
                    myclient = stream;
                    let mut stream = BufReader::new(myclient);
                    println!("Success Connecting to server");
                    let response = client::read_message(&mut stream);
                    cmd_loop(&mut stream, &arguements);
                }
                Err(_) => {
                    arguements.hostname = "".to_string();
                    arguements.ftp_port = "".to_string();
                    println!("Could not connect to host");
                }
            }
        } else {

            let (mut cmd, mut args) = get_commands();

            match cmd.to_lowercase().as_ref() {
                "open" => {
                    let (host, port) = match args.find(' ') {
                        Some(pos) => (&args[0..pos], &args[pos + 1..]),
                        None => (args.as_ref(), "21".as_ref()),
                    };

                    let server = format!("{}:{}", host, port);
                    match TcpStream::connect(server.as_str()) {
                        Ok(stream) => {
                            arguements.hostname = "".to_string();
                            arguements.ftp_port = "".to_string();
                            myclient = stream;
                            let mut stream = BufReader::new(myclient);
                            println!("Success Connecting to server");
                            let response = client::read_message(&mut stream);
                            cmd_loop(&mut stream, &arguements);
                        }
                        Err(_) => {
                            println!("Could not connect to host");
                        }

                    }
                }
                "quit" | "exit" => {
                    println!("Goodbye");
                    process::exit(1);
                }
                "close" => {
                    println!("Not Connected");
                }
                "help" => println!("{}", utils::COMMANDS_HELP),
                _ => {
                    println!("Not Connected");
                }
            }
        }
    }

}


fn login(mut client: &mut BufReader<TcpStream>, arguements: &Arguements) -> bool {
    let mut logged_in: bool = false;
    let os_user = std::env::var("USER").unwrap_or(String::new());

    let user = match arguements.username {
        Some(ref usr) => usr.to_string(),
        None => {
            print!("User ({}) ", os_user);
            io::stdout().flush().expect("Something went wrong flushing");
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Err(_) => "".to_string(),
                Ok(_) => {
                    match line.trim().is_empty() {
                        true => os_user.to_string(),
                        false => line.trim().to_string(),
                    }
                }
            }
        }
    };

    let password = match arguements.password {
        Some(ref pass) => pass.to_string(),
        None => {
            match prompt_password_stdout("Password: ") {
                Ok(pwd) => pwd.to_string(),
                Err(_) => "".to_string(),
            }
        }
    };
    let mut line = String::new();
    let mut cmd = format!("USER {}\r\n", user);
    let mut response = String::new();

    client::write_command(&mut client, &cmd);
    response = client::read_message(&mut client);

    response.clear();
    cmd = format!("PASS {}\r\n", password);

    client::write_command(&mut client, &cmd);
    response = client::read_message(&mut client);

    match client::get_code_from_respone(&response) {
        Ok(230) => {
            println!("Success Logging In");
            logged_in = true;
        }
        Ok(_) => {
            println!("Login Failed");
            logged_in = false;
        }
        Err(e) => println!("Something went wrong"),
    }

    logged_in

}



fn cmd_loop(mut client: &mut BufReader<TcpStream>, arguements: &Arguements) {

    let logged_in = login(&mut client, &arguements);
    let auth_mesg = "You need to be logged in";

    loop {
        let (cmd, args) = get_commands();
        if logged_in {
            match cmd.to_lowercase().as_ref() {
                "ls" | "list" => client::list(&mut client, &args),
                "mkdir" | "mkd" => client::make_dir(&mut client, &args),
                "cd" | "cwd" => client::change_dir(&mut client, &args),
                "dele" | "del" => client::dele(&mut client, &args),
                "cdup" | "cdu" => client::change_dir_up(&mut client),
                "pwd" => client::print_working_dir(&mut client),
                "put" | "stor" => client::put(&mut client, &args),
                "get" | "retr" => client::get(&mut client, &args),
                "rm" | "rmd" => client::remove_dir(&mut client, &args),
                "quit" | "exit" => {
                    println!("Goodbye");
                    client::quit_server(&mut client);
                    process::exit(1);
                }
                "help" => println!("{}", utils::COMMANDS_HELP),
                _ => {
                    println!("Invalid Command");
                }
            }

        } else {
            match cmd.to_lowercase().as_ref() { 
                "quit" | "exit" => {
                    println!("Goodbye");
                    client::quit_server(&mut client);
                    process::exit(1);
                }
                "help" => println!("{}", utils::COMMANDS_HELP),
                "open" => {
                    println!("Already connected, user close to end connection");
                }

                "close" => {
                    println!("Closing connection");
                    break;
                }
                _ => {
                    println!("You need to be logged in for this command");
                }

            }
        }


    }

}

fn get_commands() -> (String, String) {

    print!("ftp>");
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    let line = buf.trim();
    let (cmd, args) = match line.find(' ') {
        Some(pos) => (&line[0..pos], &line[pos + 1..]),
        None => (line, "".as_ref()),
    };

    let s1 = format!("{}", cmd);
    let s2 = format!("{}", args);

    (s1, s2)
}

fn load_defaults(settings: &mut Arguements, conf: &Ini) {
    let defaults = conf.section(Some("default".to_owned())).unwrap();

    settings.ftp_port = format!("{}",
                                defaults.get("default_ftp_port")
                                    .unwrap_or(&settings.ftp_port));
    settings.data_port_range = format!("{}-{}",
                                       defaults.get("data_port_min")
                                           .unwrap_or(&"27500".to_string()),
                                       defaults.get("data_port_max")
                                           .unwrap_or(&"2799".to_string()));
    settings.log_file = format!("{}",
                                defaults.get("default_log_file").unwrap_or(&settings.log_file));
    settings.ftp_mode = format!("{}",
                                defaults.get("default_mode").unwrap_or(&"PASSIVE".to_string()));
    match settings.ftp_mode.to_lowercase().as_ref() {
        "passive" => {
            settings.passive = true;
        }
        _ => {
            settings.passive = false;
        }
    }
}
