import json
import urllib.request
import urllib.error
import time

API_KEY = "AIzaSyAZveL0LVSpRlqgRRC79kKQAGo6OLcVSCk"
MODEL = "gemini-1.5-flash"
URL = f"https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={API_KEY}"

def run_audit():
    # Read the codebase
    try:
        with open('codebase_dump.txt', 'r', encoding='utf-8') as f:
            code_context = f.read()
    except FileNotFoundError:
        print("Error: codebase_dump.txt not found.")
        return

    # Construct Prompt
    system_prompt = """You are a world-class Safety-Critical Systems Auditor (SIL-4 certified) specializing in Rust and Python.
    Your job is to audit the provided code for the AGNIX control system.

    FOCUS AREAS:
    1. Race Conditions (Rust `Arc<Mutex<...>>` logic, Python threading).
    2. Logic Errors (Off-by-one, deadlocks, incorrect control math).
    3. Safety Bypasses (Dead code, mocks in production files, unchecked unwrap()).
    4. Integer Overflows / Floating Point issues.

    Output a report in Markdown with the following structure:
    # AGNIX AI AUDIT REPORT
    ## Critical Findings
    1. **Title**
       - **Location:** File and Line approximation.
       - **Description:** Why is this dangerous?
       - **Fix:** Code snippet or logic correction.

    Be extremely critical. If the code is safe, say so, but dig deep for "unknown unknowns"."""

    # Limit payload size if necessary (Gemini Flash has ~1M context window, so likely fine)
    # But let's check length
    print(f"Codebase length: {len(code_context)} chars")

    payload = {
        "contents": [{
            "parts": [
                {"text": system_prompt},
                {"text": f"Here is the code:\n\n{code_context}"}
            ]
        }]
    }

    data = json.dumps(payload).encode('utf-8')
    req = urllib.request.Request(URL, data=data, headers={'Content-Type': 'application/json'})

    print("Sending request to Gemini...")
    try:
        with urllib.request.urlopen(req) as response:
            result = json.load(response)

            # Extract text
            try:
                report_text = result['candidates'][0]['content']['parts'][0]['text']

                with open('gemini_audit_report.md', 'w', encoding='utf-8') as f:
                    f.write(report_text)
                print("Audit Complete. Report saved to gemini_audit_report.md")
                print("-" * 20)
                print(report_text)
                print("-" * 20)
            except (KeyError, IndexError) as e:
                print("Error parsing response:", result)

    except urllib.error.HTTPError as e:
        print(f"HTTP Error: {e.code} {e.reason}")
        print(e.read().decode('utf-8'))
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    run_audit()
