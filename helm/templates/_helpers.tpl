{{/*
Expand the name of the chart.
*/}}
{{- define "cuscuta.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Expand the full name of the chart.
*/}}
{{- define "cuscuta.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}


{{/*
Common labels
*/}}
{{- define "cuscuta.labels" -}}
helm.sh/chart: {{ include "cuscuta.chart" . }}
{{ include "cuscuta.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "cuscuta.selectorLabels" -}}
app.kubernetes.io/name: {{ include "cuscuta.fullname" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "cuscuta.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Full image name for a component.
Usage: {{ include "cuscuta.image" (dict "root" . "component" "cuscuta-entry") }}
*/}}
{{- define "cuscuta.image" -}}
{{- $tag := .root.Values.image.tag | toString }}
{{- printf "%s/%s:%s" .root.Values.image.repository .component $tag }}
{{- end }}

{{/*
Component labels (appends component name to selector labels).
Usage: {{ include "cuscuta.componentLabels" (dict "root" . "component" "entry") }}
*/}}
{{- define "cuscuta.componentLabels" -}}
{{ include "cuscuta.selectorLabels" .root }}
app.kubernetes.io/component: {{ .component }}
{{- end }}

{{/*
Chilo service URL (used by worker).
*/}}
{{- define "cuscuta.chiloUrl" -}}
{{- printf "http://%s-chilo:%d/generate" (include "cuscuta.fullname" .) (.Values.chilo.port | int) }}
{{- end }}

{{/*
Resolve a Secret name. If the external secret is enabled, use its name;
otherwise fall back to the chart-owned Secret.
Usage: {{ include "cuscuta.secretName" (dict "root" . "cfg" .Values.postgresql.secret) }}
*/}}
{{- define "cuscuta.secretName" -}}
{{- if .cfg.enabled }}
{{- .cfg.name }}
{{- else }}
{{- printf "%s-secret" (include "cuscuta.fullname" .root) }}
{{- end }}
{{- end }}

{{/*
Resolve a Secret key. If external secret is enabled, use its configured key;
otherwise use the default key name.
Usage: {{ include "cuscuta.secretKey" (dict "cfg" .Values.postgresql.secret "defaultKey" "ACCOUNTS_SQL_ADDR") }}
*/}}
{{- define "cuscuta.secretKey" -}}
{{- if .cfg.enabled }}
{{- .cfg.key }}
{{- else }}
{{- .defaultKey }}
{{- end }}
{{- end }}