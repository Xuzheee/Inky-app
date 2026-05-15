import { invoke } from '@tauri-apps/api/core';
import { builtInPetSets, localPackToPetSet, type LocalPetPack, type PetSetConfig } from '../components/PetRenderer/petConfig';

export async function loadPetSets(): Promise<PetSetConfig[]> {
  try {
    const localPacks = await invoke<LocalPetPack[]>('list_pet_packs');

    return [...builtInPetSets, ...localPacks.map(localPackToPetSet)];
  } catch {
    return builtInPetSets;
  }
}

export async function openPetPacksFolder(): Promise<string | null> {
  try {
    return await invoke<string>('open_pet_packs_folder');
  } catch {
    return null;
  }
}
