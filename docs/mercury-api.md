# Mercury API Reference

## Endpoints

### Chat Completions
- `POST /v1/chat/completions`
- Models: `mercury-2`, `mercury-edit`
- Supports: tool calling, structured outputs (json_schema), streaming

### FIM / Autocomplete
- `POST /v1/fim/completions`
- Model: `mercury-edit`
- Fields: `prompt`, `suffix`

### Next Edit
- `POST /v1/edit/completions`
- Model: `mercury-edit`
- Format: recently_viewed_snippets + current_file_content (with code_to_edit region) + edit_diff_history
- Best practices: 3-5 snippets ~20 lines each, last 3-5 user edits, 10-15 line editable region

### Apply Edit
- `POST /v1/apply/completions`
- Model: `mercury-edit`
- Format: original_code + update_snippet (with `// ... existing code ...` markers)

## Tool Calling (Chat Completions)
Standard OpenAI-compatible tool calling format:
```json
{
  "model": "mercury-2",
  "messages": [...],
  "tools": [{
    "type": "function",
    "function": {
      "name": "tool_name",
      "description": "...",
      "parameters": { "type": "object", "properties": {...}, "required": [...] }
    }
  }]
}
```

Response includes `tool_calls` array in message:
```json
{
  "choices": [{
    "message": {
      "tool_calls": [{
        "function": {
          "name": "tool_name",
          "arguments": "{...}"
        }
      }]
    }
  }]
}
```

## Structured Outputs
```json
{
  "response_format": {
    "type": "json_schema",
    "json_schema": {
      "name": "SchemaName",
      "strict": true,
      "schema": { ... }
    }
  }
}
```

## Next Edit Format
```
<|recently_viewed_code_snippets|>
<|recently_viewed_code_snippet|>
code_snippet_file_path: [PATH]
[CODE]
<|/recently_viewed_code_snippet|>
<|/recently_viewed_code_snippets|>

<|current_file_content|>
current_file_path: [PATH]
[CODE ABOVE]
<|code_to_edit|>
[EDITABLE REGION]
<|/code_to_edit|>
[CODE BELOW]
<|/current_file_content|>

<|edit_diff_history|>
--- [PATH]
+++ [PATH]
@@ ... @@
[DIFF]
<|/edit_diff_history|>
```

## Apply Edit Format
```
<|original_code|>
{original_code}
<|/original_code|>

<|update_snippet|>
// ... existing code ...
[UPDATED CODE]
// ... existing code ...
<|/update_snippet|>
```

## Models
- `mercury-2` — reasoning model, uses reasoning_tokens, good for chat + tool calling
- `mercury-edit` — code editing model, FIM/next-edit/apply-edit
- `mercury-coder` — gated to pre-Feb 2026 accounts
