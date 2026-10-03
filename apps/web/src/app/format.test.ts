import { describe, expect, it } from 'vitest';
import { ProjectSettings } from '../model/projectSettings';
import { Formatter } from './format';

/**
 * The type's axes (docs/adr/0165 §4): a CAD project names east X and north Y, its polar angles from east; a CBS
 * project and one not asked its type the surveyor's Y and X and semt. The Hesap windows' stay the surveyor's. The
 * desktop's `Format` says the same (crates/native/interaction/src/format.rs).
 */
describe('Formatter: the project type’s axes', () => {
  it('names east and north, the typed forms and the polar angle as the type does', () => {
    const settings = new ProjectSettings({ workspace: 'cad', angleUnit: 'deg' });
    const cad = new Formatter(settings);
    expect(cad.point({ x: 120, y: 45.5 })).toBe('X 120.000  Y 45.500');
    expect(cad.inputHint).toBe('mesafe · X,Y · @dX,dY · @mesafe<açı');
    expect(cad.axesText('Taban noktasına tıklayın ya da Y,X yazın; aralık dY,dX. Y (sağa), X (yukarı)')).toBe(
      'Taban noktasına tıklayın ya da X,Y yazın; aralık dX,dY. X (sağa), Y (yukarı)',
    );
    expect(cad.angles).toEqual({ fromNorth: false, grads: false });
    expect(cad.metric().pairLabel, 'the surveyor’s').toBe('Y,X');
    // A direction from east, counter-clockwise: semt 0 g (north) is 90°, 300 g (west) 180°.
    expect([cad.directionName, cad.direction(0), cad.direction(300), cad.direction(100, false)]).toEqual(['Açı', '90.0000°', '180.0000°', '0.0000']);

    settings.assign({ workspace: 'gis', angleUnit: 'grad' });
    expect(cad.inputHint).toBe('mesafe · Y,X · @dY,dX · @mesafe<semt');
    expect(cad.axesText('Y,X')).toBe('Y,X');
    expect([cad.directionName, cad.direction(300)]).toEqual(['Semt', '300.0000 g']);
    expect(cad.angles).toEqual({ fromNorth: true, grads: true });

    // Not asked its type: shown as CBS.
    settings.workspace.set(null);
    expect([cad.eastLabel, cad.northLabel]).toEqual(['Y', 'X']);
  });
});
