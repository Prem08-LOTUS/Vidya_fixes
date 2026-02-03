import json
import urllib.request
import urllib.error
import time

# Try gemini-pro which is often the default free one
API_KEY = "AIzaSyAZveL0LVSpRlqgRRC79kKQAGo6OLcVSCk"
# "GEMINI 3" might refer to something else, but let's try gemini-1.5-pro-latest or gemini-pro
MODELS_TO_TRY = ["gemini-1.5-flash", "gemini-1.5-pro", "gemini-pro"]

def run_audit():
    # Read the codebase
    try:
        with open('codebase_dump.txt', 'r', encoding='utf-8') as f:
            code_context = f.read()
    except FileNotFoundError:
        print("Error: codebase_dump.txt not found.")
        return

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

    payload = {
        "contents": [{
            "parts": [
                {"text": system_prompt},
                {"text": f"Here is the code:\n\n{code_context[:500000]}"} # Limit just in case
            ]
        }]
    }

    data = json.dumps(payload).encode('utf-8')

    # Try models
    # The previous error was 404 for gemini-1.5-flash on v1beta.
    # Let's try v1beta with gemini-pro, or check if we need to use a different endpoint.
    # The error message said "Call ListModels".
    # I will try 'gemini-1.5-flash-latest' and 'gemini-pro'.

    # Actually, standard free tier usually supports gemini-1.5-flash.
    # The URL structure might need /v1beta/models/...

    for model in ["gemini-1.5-flash-latest", "gemini-1.5-flash", "gemini-pro"]:
        print(f"Trying model: {model}")
        url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={API_KEY}"

        req = urllib.request.Request(url, data=data, headers={'Content-Type': 'application/json'})

        try:
            with urllib.request.urlopen(req) as response:
                result = json.load(response)
                # Extract text
                try:
                    report_text = result['candidates'][0]['content']['parts'][0]['text']

                    with open('gemini_audit_report.md', 'w', encoding='utf-8') as f:
                        f.write(report_text)
                    print(f"Audit Complete using {model}. Report saved to gemini_audit_report.md")
                    return
                except (KeyError, IndexError) as e:
                    print("Error parsing response:", result)
        except urllib.error.HTTPError as e:
            print(f"HTTP Error with {model}: {e.code} {e.reason}")
            # print(e.read().decode('utf-8'))
        except Exception as e:
            print(f"Error: {e}")

if __name__ == "__main__":
    run_audit()
