import type { ProcessingModel } from '../model';

/**
 * Models that ship with KentOS: ready to run and a starting point to copy
 * in the model designer. They cannot be changed in place (a copy can).
 */
export const BUILTIN_MODELS: readonly ProcessingModel[] = [
  {
    id: 'builtin.parcelSheet',
    label: 'Parsel ölçü yazıları',
    category: 'cadastre',
    description: 'Parsellerin köşelerini numaralar, kenar uzunluklarını yazar ve hesaplanan alanı özniteliğe yazar; hepsi tek adımda geri alınır.',
    inputs: [
      { name: 'parcels', label: 'Parseller', type: 'features', kinds: ['polygon'], default: { scope: 'selection' }, description: 'Ölçü yazıları hazırlanacak parseller.' },
      { name: 'prefix', label: 'Nokta öneki', type: 'string', default: 'P', allowEmpty: true, maxLength: 12, description: 'Köşe numaralarının başındaki yazı: P00001.' },
    ],
    steps: [
      { id: 'corners', tool: 'points.numberVertices', values: { input: { kind: 'input', name: 'parcels' }, prefix: { kind: 'input', name: 'prefix' } }, position: { x: 320, y: 60 } },
      { id: 'edges', tool: 'annotation.edgeLengths', values: { input: { kind: 'input', name: 'parcels' } }, position: { x: 320, y: 180 } },
      {
        id: 'area',
        tool: 'attributes.calculate',
        values: { input: { kind: 'input', name: 'parcels' }, field: { kind: 'value', value: 'Hesap alanı' }, value: { kind: 'value', value: 'metin($alan, 2)' }, label: { kind: 'value', value: false } },
        position: { x: 320, y: 300 },
      },
    ],
    outputs: [{ name: 'points', label: 'Köşe noktaları', from: { step: 'corners', output: 'points' } }],
  },
];
