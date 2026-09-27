import { describe, expect, it } from 'vitest';
import type { Entity } from '../../model/entities';
import { readPolar, readStakeout, resolvePointIn, type PolarForm, type Row } from './read';

/**
 * Kutupsal alım and Aplikasyon read their fields and ask the geometry core
 * (the real one, through WASM): the messages name what the user sees.
 */

const resolve = (text: string) => resolvePointIn({ all: () => ([] as Entity[]).values() }, text);

const polar = (rows: Row[], extra: Partial<PolarForm> = {}): PolarForm => ({ station: '0,0', back: '0,100', backReading: '0', stationZ: '', instrumentHeight: '', rows, ...extra });

describe('Kutupsal alım', () => {
  it('a core message names the table row, not the shot counted among the filled rows', () => {
    const rows: Row[] = [{ name: '1', reading: '0', distance: '10' }, {}, { name: '3', reading: '100', distance: '0' }];
    const read = readPolar(polar(rows), resolve, 'grad');
    expect(read.points).toBeNull();
    expect(read.errors).toEqual(['3. noktanın uzunluğu sıfırdan büyük olmalı.']);
    // The zenith's message too: a zenith of 0 leaves no horizontal distance.
    const zenith = readPolar(polar([{}, {}, { reading: '0', distance: '10', zenith: '0' }]), resolve, 'grad');
    expect(zenith.errors).toEqual(['3. noktanın başucu açısı yatay uzunluk bırakmıyor (0 ile yarım tur arasında olmalı).']);
  });

  it('a station height or an instrument height that is not a number is refused, as the other fields are', () => {
    const read = readPolar(polar([{ reading: '0', distance: '10' }], { stationZ: '12a', instrumentHeight: 'x' }), resolve, 'grad');
    expect(read.points).toBeNull();
    expect(read.errors).toEqual(['İstasyon kotu bir sayı değil.', 'Alet yüksekliği bir sayı değil.']);
  });

  it('numbers are taken: a slope distance at a quarter turn is horizontal, the height from the station’s', () => {
    const read = readPolar(polar([{ name: 'A', reading: '100', distance: '10', zenith: '100', target: '1.5' }], { stationZ: '100', instrumentHeight: '1,5' }), resolve, 'grad');
    expect(read.errors).toEqual([]);
    expect(read.names).toEqual(['A']);
    expect(read.points).toHaveLength(1);
    const [p] = read.points!;
    expect(p.p.x).toBeCloseTo(10, 9);
    expect(p.p.y).toBeCloseTo(0, 9);
    expect(p.z).toBeCloseTo(100, 9);
  });
});

describe('Aplikasyon', () => {
  it('a target at the station is a row error: it has no bearing', () => {
    const read = readStakeout({ station: '10,20', back: '', rows: [{ point: '10,20' }, { point: '10,30' }] }, resolve, 'grad');
    expect(read.stakes).toBeNull();
    expect(read.errors).toEqual(['1. satırdaki nokta durulan noktayla aynı yerde; semt tanımsız.']);
  });

  it('the other targets have their bearing and distance', () => {
    const read = readStakeout({ station: '10,20', back: '', rows: [{}, { point: '10,30' }] }, resolve, 'grad');
    expect(read.errors).toEqual([]);
    expect(read.back).toBe(false);
    expect(read.names).toEqual(['10,30']);
    expect(read.stakes).toHaveLength(1);
    expect(read.stakes![0].bearing).toBeCloseTo(0, 12);
    expect(read.stakes![0].distance).toBeCloseTo(10, 12);
  });
});
