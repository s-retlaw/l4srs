# l4srs
Rust implementation of the Log 4 Shell (log 4 j - CVE-2021-44228)

This version will let you build command classes, dynamically serve
a mini meterpreter that runs in a thread of the exploited Java app,
and runs the LDAP and HTTP servers.  This version servers both the
LDAP and HTTP requests from the same port.

You can run on multiple ports simultaneously to attempt to see 
what ports may be available for egress on for the target machine.

If the request is not LDAP or HTTP it can then proxy the request
to another machine, again on the same port.  If the target machine
has only one egress port you can server LDAP, HTTP and use the same
port to proxy the meterpreter connection to another local port or
another machine.

**This version no longer requires javac to be installed.  It still
allows for dynamic "class building" but does so by altering 
precompiled classes embeded in the executable.**

This version adds the top 100 and top 1000 ports as defined by nmap.
use the --pC100 or the --pC1000 options.

Typical use case is to build command class(es) then run the server.
`l4srs build -c Cmd1 -l firefox -w Calc.exe`
`l4srs build -c TouchMe -l "touch /tmp/me"`

you can then request Cmd1 and this will launch firefox on linux 
and Calc on Windows.  It you request TouchMe it will touch 
/tmp/me on linux and on windows it will not execute anything.

you can then run `l4srs run --pC100` to start the server on the
top 100 ports and can serve Cmd1 or TouchMe.

Additionaly if you request MM:Host:port it will dynamically
create a mini meterpreter class that will reach out to the
host and port in the request.  If your msfconsole is running
on 10.20.30.40 on port 4444 you would request MM:10.20.30.40:4444
this is not built with the build command it is dymaically built
on the request.

This version allows for the building and serving of classes from
an in memory cache.  All of the dynamic MiniMeterpreter classes
no longer touch the file system.  Additionally 2 new flags are
added. The first is --no_fs which will enforce that we never
server any files from the file system.  The other --allow_cmd
will enable dynamic class build by hitting /build_cmd from any
open port with a post request and a json body with fields :
class_name, l_cmd, w_cmd
