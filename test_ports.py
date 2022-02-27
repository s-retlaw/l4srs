import sys
import json
import threading
import subprocess
import time

def is_a_match(line):
    try:
        return json.loads(line)['port'] > 0
    except Exception :
        return False

def send_requests(ports, cmd_line):
    for port in ports:
        cmd_line = list(map(lambda s: s.replace("$PORT$", port), cmd_line))
        subprocess.run(cmd_line, stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
        time.sleep(1000)

    print("tried all ports....sleeping 5 seconds")
    sys.exit(0)

def run(cmd_line):
    ports = None
    for line in sys.stdin:
        if line.startswith("Running on : ") :
            ports = line.split(":")[1].strip().split(",")
            print("The ports are : ", ports)
            break

    if ports is None :
        sys.exit("Unexpected stdin inputs ... exiting")

    t = threading.Thread(target=send_requests, args=(ports, cmd_line))
    t.start()

    for line in sys.stdin:
        if is_a_match(line) :
            print("We have a hit : "+line);
            sys.exit(0)

def main():
    if len(sys.argv) == 1:
        print(f'Usage {sys.argv[0]} <command>.  Note the port in the command will replace the $PORT$ var')
        sys.exit(-1)

    run(sys.argv[1:])

if __name__ == "__main__" :
    main()
