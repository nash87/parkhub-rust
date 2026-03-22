import { useEffect, useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { MapPin, NavigationArrow } from '@phosphor-icons/react';
import { MapContainer, TileLayer, Marker, Popup, useMap } from 'react-leaflet';
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';
import { api, type LotMarker } from '../api/client';
import { staggerSlow, fadeUp } from '../constants/animations';

// Fix Leaflet default icon path issue with bundlers
delete (L.Icon.Default.prototype as any)._getIconUrl;
L.Icon.Default.mergeOptions({
  iconRetinaUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-icon-2x.png',
  iconUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-icon.png',
  shadowUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-shadow.png',
});

const MARKER_COLORS: Record<string, string> = {
  green: '#22c55e',
  yellow: '#eab308',
  red: '#ef4444',
  gray: '#6b7280',
};

function createColorIcon(color: string): L.DivIcon {
  const hex = MARKER_COLORS[color] || MARKER_COLORS.gray;
  return L.divIcon({
    className: 'custom-marker',
    html: `<div style="
      width: 28px; height: 28px; border-radius: 50% 50% 50% 0;
      background: ${hex}; transform: rotate(-45deg);
      border: 3px solid white; box-shadow: 0 2px 8px rgba(0,0,0,0.3);
    "><div style="
      width: 10px; height: 10px; border-radius: 50%;
      background: white; position: absolute;
      top: 50%; left: 50%; transform: translate(-50%, -50%);
    "></div></div>`,
    iconSize: [28, 28],
    iconAnchor: [14, 28],
    popupAnchor: [0, -28],
  });
}

/** Auto-fit the map to show all markers */
function FitBounds({ markers }: { markers: LotMarker[] }) {
  const map = useMap();
  useEffect(() => {
    if (markers.length === 0) return;
    const bounds = L.latLngBounds(markers.map(m => [m.latitude, m.longitude]));
    map.fitBounds(bounds, { padding: [40, 40], maxZoom: 15 });
  }, [markers, map]);
  return null;
}

export function MapViewPage() {
  const { t } = useTranslation();
  const [markers, setMarkers] = useState<LotMarker[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.getMapMarkers()
      .then(res => {
        if (res.success && res.data) setMarkers(res.data);
      })
      .finally(() => setLoading(false));
  }, []);

  const icons = useMemo(() => ({
    green: createColorIcon('green'),
    yellow: createColorIcon('yellow'),
    red: createColorIcon('red'),
    gray: createColorIcon('gray'),
  }), []);

  const container = staggerSlow;
  const item = fadeUp;

  // Default center: Munich, Germany (can be overridden by markers)
  const defaultCenter: [number, number] = [48.1351, 11.5820];

  if (loading) {
    return (
      <div className="space-y-6">
        <div className="h-10 w-64 skeleton rounded-xl" />
        <div className="h-[500px] skeleton rounded-2xl" />
      </div>
    );
  }

  return (
    <motion.div variants={container} initial="hidden" animate="show" className="space-y-6">
      <motion.div variants={item}>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-3">
          <MapPin weight="fill" className="w-7 h-7 text-primary-500" />
          {t('map.title')}
        </h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">
          {t('map.subtitle')}
        </p>
      </motion.div>

      {markers.length === 0 ? (
        <motion.div variants={item} className="card p-12 text-center">
          <NavigationArrow weight="light" className="w-16 h-16 mx-auto text-surface-300 dark:text-surface-600 mb-4" />
          <p className="text-surface-500 dark:text-surface-400 text-lg">{t('map.noLots')}</p>
        </motion.div>
      ) : (
        <motion.div variants={item} className="card overflow-hidden rounded-2xl" data-testid="map-container">
          <MapContainer
            center={defaultCenter}
            zoom={13}
            scrollWheelZoom
            style={{ height: '500px', width: '100%' }}
          >
            <TileLayer
              attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
              url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
            />
            <FitBounds markers={markers} />
            {markers.map(marker => (
              <Marker
                key={marker.id}
                position={[marker.latitude, marker.longitude]}
                icon={icons[marker.color] || icons.gray}
              >
                <Popup>
                  <div className="min-w-[200px] p-1">
                    <h3 className="font-bold text-base mb-1">{marker.name}</h3>
                    <p className="text-sm text-gray-600 mb-2">{marker.address}</p>
                    <div className="flex items-center justify-between mb-3">
                      <span className="text-sm font-medium">
                        {t('map.available')}: {marker.available_slots}/{marker.total_slots}
                      </span>
                      <span
                        className={`px-2 py-0.5 rounded-full text-xs font-medium text-white`}
                        style={{ backgroundColor: MARKER_COLORS[marker.color] }}
                      >
                        {marker.status}
                      </span>
                    </div>
                    <a
                      href="/book"
                      className="block w-full text-center px-4 py-2 bg-primary-600 text-white rounded-lg text-sm font-medium hover:bg-primary-700 transition-colors"
                    >
                      {t('map.bookNow')}
                    </a>
                  </div>
                </Popup>
              </Marker>
            ))}
          </MapContainer>

          {/* Legend */}
          <div className="flex items-center gap-4 px-4 py-3 bg-surface-50 dark:bg-surface-800/50 border-t border-surface-200 dark:border-surface-700">
            {[
              { color: 'green', label: '> 50%' },
              { color: 'yellow', label: '10-50%' },
              { color: 'red', label: '< 10%' },
              { color: 'gray', label: t('map.closed') },
            ].map(item => (
              <div key={item.color} className="flex items-center gap-1.5 text-xs text-surface-600 dark:text-surface-400">
                <span
                  className="w-3 h-3 rounded-full"
                  style={{ backgroundColor: MARKER_COLORS[item.color] }}
                />
                {item.label}
              </div>
            ))}
          </div>
        </motion.div>
      )}
    </motion.div>
  );
}
