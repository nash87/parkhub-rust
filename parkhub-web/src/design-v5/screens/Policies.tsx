import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, SectionLabel, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type Policy } from '../../api/client';
import type { ScreenId } from '../nav';

function formatWhen(iso: string): string {
  return new Date(iso).toLocaleString('de-DE', {
    day: '2-digit', month: '2-digit', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  });
}

export function PoliciesV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [activeId, setActiveId] = useState<string | null>(null);
  const [draft, setDraft] = useState('');
  const [preview, setPreview] = useState(false);

  const { data: policies = [], isLoading, isError } = useQuery({
    queryKey: ['policies'],
    queryFn: async () => {
      const res = await api.getPolicies();
      if (!res.success) throw new Error(res.error?.message ?? 'Richtlinien konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });

  useEffect(() => {
    if (!activeId && policies.length > 0) setActiveId(policies[0].id);
  }, [policies, activeId]);

  const active = policies.find((p) => p.id === activeId) ?? null;

  useEffect(() => {
    const p = policies.find((x) => x.id === activeId);
    if (p) { setDraft(p.body); setPreview(false); }
    // Only depend on activeId: refetches produce a new policies array, which
    // would otherwise clobber the user's in-flight draft on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeId]);

  const save = useMutation({
    mutationFn: async (payload: { id: string; body: string }) => {
      const res = await api.updatePolicy(payload.id, payload.body);
      if (!res.success) throw new Error(res.error?.message ?? 'Speichern fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['policies'] });
      toast('Richtlinie gespeichert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Speichern fehlgeschlagen', 'error'),
  });

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 400, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite' }} />
      </div>
    );
  }

  if (isError) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
        </Card>
      </div>
    );
  }

  const isDirty = active && draft !== active.body;

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Richtlinien</div>

      {policies.length === 0 ? (
        <Card className="v5-ani" style={{ padding: 36, textAlign: 'center' }}>
          <V5NamedIcon name="shield" size={20} color="var(--v5-mut)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Keine Richtlinien</div>
        </Card>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: '220px 1fr', gap: 12, flex: 1, minHeight: 0 }}>
          <Card className="v5-ani" style={{ padding: 8, overflow: 'auto' }}>
            <SectionLabel>Dokumente</SectionLabel>
            {policies.map((p: Policy) => {
              const isActive = p.id === activeId;
              return (
                <button
                  key={p.id} type="button"
                  onClick={() => setActiveId(p.id)}
                  data-testid="policies-nav"
                  style={{
                    display: 'block', width: '100%', textAlign: 'left',
                    padding: '8px 10px', borderRadius: 8, marginBottom: 3,
                    background: isActive ? 'var(--v5-acc-muted)' : 'transparent',
                    color: isActive ? 'var(--v5-acc)' : 'var(--v5-txt)',
                    border: 'none', cursor: 'pointer', fontSize: 12, fontWeight: isActive ? 600 : 500,
                  }}
                >{p.title}</button>
              );
            })}
          </Card>

          <Card className="v5-ani" style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10, minHeight: 0 }}>
            {active ? (
              <>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <div>
                    <div style={{ fontWeight: 600, fontSize: 14, color: 'var(--v5-txt)' }}>{active.title}</div>
                    <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>
                      Zuletzt aktualisiert: {formatWhen(active.updated_at)}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 6 }}>
                    <button
                      type="button"
                      onClick={() => setPreview((p) => !p)}
                      data-testid="policies-preview-toggle"
                      aria-pressed={preview}
                      style={{ padding: '6px 12px', borderRadius: 8, background: preview ? 'var(--v5-acc-muted)' : 'var(--v5-sur2)', border: `1px solid ${preview ? 'var(--v5-acc)' : 'var(--v5-bor)'}`, color: preview ? 'var(--v5-acc)' : 'var(--v5-txt)', fontSize: 11, fontWeight: 500, cursor: 'pointer' }}
                    >{preview ? 'Bearbeiten' : 'Vorschau'}</button>
                    <button
                      type="button"
                      disabled={!isDirty || save.isPending}
                      onClick={() => save.mutate({ id: active.id, body: draft })}
                      data-testid="policies-save"
                      style={{
                        padding: '6px 14px', borderRadius: 8, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
                        border: 'none', fontSize: 11, fontWeight: 600,
                        cursor: isDirty && !save.isPending ? 'pointer' : 'not-allowed',
                        opacity: isDirty && !save.isPending ? 1 : 0.5,
                      }}
                    >{save.isPending ? 'Speichert …' : 'Speichern'}</button>
                  </div>
                </div>

                {preview ? (
                  <div
                    data-testid="policies-preview"
                    style={{
                      flex: 1, overflow: 'auto', padding: 14, borderRadius: 9,
                      background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)',
                      fontSize: 12, color: 'var(--v5-txt)', whiteSpace: 'pre-wrap',
                      fontFamily: 'inherit', lineHeight: 1.55,
                    }}
                  >{draft || '(leer)'}</div>
                ) : (
                  <textarea
                    data-testid="policies-editor"
                    value={draft}
                    onChange={(e) => setDraft(e.target.value)}
                    style={{
                      flex: 1, padding: 12, borderRadius: 9,
                      background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)',
                      color: 'var(--v5-txt)', fontSize: 12, lineHeight: 1.55,
                      outline: 'none', resize: 'none',
                      fontFamily: "ui-monospace, 'SFMono-Regular', Menlo, monospace",
                    }}
                  />
                )}

                <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>
                  Hinweis: Markdown-Renderer folgt in separater PR; aktuell Klartext-Vorschau.
                </div>
              </>
            ) : null}
          </Card>
        </div>
      )}
    </div>
  );
}
