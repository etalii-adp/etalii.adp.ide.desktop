{{- define "shop.fullname" -}}
{{ .Release.Name }}-shop
{{- end }}
{{- define "shop.labels" -}}
app: shop
{{- end }}
