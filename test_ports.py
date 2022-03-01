import sys
import json
import threading
import subprocess
import time
import os

def is_a_match(line):
    try:
        return json.loads(line)['port'] > 0
    except Exception :
        return False

def exit(msg=""):
    print(msg)
    print("CTRl C to kill l4srs")

    try:
        os._exit(0)
    except Exception:
        sys.exit()


def send_requests(ports, cmd_line):
    for port in ports:
        cl = list(map(lambda s: f'{s}', cmd_line))
        cl = list(map(lambda s: s.replace("$PORT$", port), cl))
        try:
            print(f'About to run : {cl}')
            subprocess.run(cl, stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL, timeout=2)
            time.sleep(1000)
        except Exception:
            print("timedout...")
    print("tried all ports....sleeping 5 seconds")
    time.sleep(5000)
    exit()

def run(cmd_line):
    ports = None
    for line in sys.stdin:
        if line.startswith("Running on : ") :
            ports = line.split(":")[1].strip().split(",")
            print("The ports are : ", ports)
            break

    if ports is None :
        exit("Unexpected stdin inputs ... exiting")

    t = threading.Thread(target=send_requests, args=(ports, cmd_line))
    t.start()

    for line in sys.stdin:
        if is_a_match(line) :
            print("We have a hit : "+line);
            exit()

def main():
    if len(sys.argv) == 1:
        print(f'Usage {sys.argv[0]} <command>.  Note the port in the command will replace the $PORT$ var')
        sys.exit(-1)

    run(sys.argv[1:])

if __name__ == "__main__" :
    main()
