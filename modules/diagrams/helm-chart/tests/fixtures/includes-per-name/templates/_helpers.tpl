{{- define "parity.fullname" -}}
{{ .Release.Name }}-{{ .Chart.Name }}
{{- end -}}

{{- define "parity.labels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
{{- end -}}
