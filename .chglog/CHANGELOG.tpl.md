{{ range .Versions }}
## interBTC {{ if .Tag.Previous }}[{{ .Tag.Name }}]({{ $.Info.RepositoryURL }}/compare/{{ .Tag.Previous.Name }}...{{ .Tag.Name }}){{ else }}{{ .Tag.Name }}{{ end }} ({{ datetime "2006-01-02" .Tag.Date }})
*This release contains the changes from {{ .Tag.Previous.Name }} to {{ .Tag.Name }}.*

## Global Priority
- 🔴 HIGH: This is a **high priority** release and you must upgrade as soon as possible.
- 🔵 MEDIUM: This is **medium priority** release and you should upgrade not later than [INSERT DATE]
- ⚪ LOW: This is a **low priority** release and you may upgrade at your convenience.


## Breaking CLI changes

{{ if .MergeCommits }}
## Changes
{{ range .MergeCommits -}}
- {{ .TrimmedBody }} [#{{ .Merge.Source  }}](https://github.com/interlay/interbtc/issues/{{ .Merge.Source  }})
{{ end }}
{{ end -}}


{{ range .CommitGroups -}}
### {{ .Title }}

{{ range .Commits -}}
* {{ .Subject }}
{{ end }}
{{ end -}}

{{- if .NoteGroups -}}
{{ range .NoteGroups -}}
### {{ .Title }}

{{ range .Notes }}
{{ .Body }}
{{ end }}
{{ end -}}
{{ end -}}
{{ end -}}
