{{- define "parity.fullname" -}}
{{ .Chart.Name }}
{{- end -}}

{{- define "parity.serviceAccount" -}}
{{ include "parity.labels" . | sha256sum | trunc 8 }}
{{- end -}}
