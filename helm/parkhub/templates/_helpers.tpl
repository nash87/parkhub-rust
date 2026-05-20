{{/*
Expand the name of the chart.
*/}}
{{- define "parkhub.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a fully qualified app name.
*/}}
{{- define "parkhub.fullname" -}}
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
Create chart name and version as used by the chart label.
*/}}
{{- define "parkhub.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "parkhub.labels" -}}
helm.sh/chart: {{ include "parkhub.chart" . }}
{{ include "parkhub.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "parkhub.selectorLabels" -}}
app.kubernetes.io/name: {{ include "parkhub.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Image tag — defaults to appVersion
*/}}
{{- define "parkhub.imageTag" -}}
{{- default .Chart.AppVersion .Values.image.tag }}
{{- end }}

{{/*
Image reference — tag by default, digest when image.digest is set.
*/}}
{{- define "parkhub.imageRef" -}}
{{- $repository := required "image.repository is required" .Values.image.repository -}}
{{- $digest := trimPrefix "@" (trim (default "" .Values.image.digest)) -}}
{{- if $digest -}}
{{- if not (hasPrefix "sha256:" $digest) -}}
{{- fail "image.digest must be empty or start with sha256:" -}}
{{- end -}}
{{- printf "%s@%s" $repository $digest -}}
{{- else -}}
{{- printf "%s:%s" $repository (include "parkhub.imageTag" .) -}}
{{- end -}}
{{- end }}
