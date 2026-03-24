schema_version: "2.1"
_type: repo_overview
framework: "Rust (binary)"
patterns[2]: Async/concurrent,"Data serialization"
modules[22]{name,purpose,files,risk}:
  ..docs,"..docs module",1,low
  semfora,"semfora module",1,high
  root,"root module",1,high
  api,"API route handlers",2,high
  event,"event module",1,high
  session,"session module",1,high
  tools,"tools module",7,high
  app,"app module",1,high
  plan,"plan module",1,high
  Cargo,"Cargo module",1,low
  ui,"ui module",7,high
  logger,"logger module",1,high
  config,"Configuration files",1,high
  ROADMAP,"ROADMAP module",1,low
  audit_report,"audit_report module",1,low
  complexity_report,"complexity_report module",1,low
  proposed_changes,"proposed_changes module",1,low
  CLAUDE,"CLAUDE module",1,low
  context,"context module",2,high
  README,"README module",1,low
  refactor_plan,"refactor_plan module",1,low
  commands,"commands module",1,high
files: 36
risk_breakdown: "high:23,medium:1,low:12"
entry_points[1]: ./src/main.rs