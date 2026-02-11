import subprocess
import signal
import sys
import time
import os

# FIX #97: The "Zombie Child"
# Auto-reap child processes to prevent process table exhaustion
def reap_zombies(signum, frame):
    while True:
        try:
            pid, status = os.waitpid(-1, os.WNOHANG)
            if pid == 0:
                break
        except ChildProcessError:
            break

signal.signal(signal.SIGCHLD, reap_zombies)

def run_agnix(job_id):
    # FIX #93: The "Shell Injection" Hole
    # NEVER use shell=True. Pass arguments as a list.
    cmd = ["./target/release/agnix", "--job", str(job_id)]

    print(f"[ORCHESTRATOR] Starting Job {job_id} safely...")

    try:
        # Popen without shell=True avoids injection
        process = subprocess.Popen(cmd)
        return process
    except FileNotFoundError:
        print("Error: agnix binary not found. Build it first!")
        return None

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 orchestrator.py <job_id>")
        sys.exit(1)

    job_id = sys.argv[1]

    # Validation (Defense in Depth)
    if not job_id.isalnum():
        print("Security Alert: Invalid Job ID")
        sys.exit(1)

    proc = run_agnix(job_id)

    if proc:
        try:
            proc.wait() # Explicit wait (though signal handler helps too)
        except KeyboardInterrupt:
            print("Stopping...")
            proc.terminate()
