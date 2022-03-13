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

This version no longer requires javac to be installed.  It still
allows for dynamic "class building" but does so by altering 
precompiled classes embeded in the executable.

