import { useState } from 'react';
import * as ImagePicker from 'expo-image-picker';

export interface ImagePickerResult {
  uri: string;
  width: number;
  height: number;
  type?: string;
  fileName?: string;
}

export interface UseImagePickerOptions {
  allowsEditing?: boolean;
  aspect?: [number, number];
  quality?: number;
  allowsMultipleSelection?: boolean;
}

export const useImagePicker = (options: UseImagePickerOptions = {}) => {
  const {
    allowsEditing = false,
    aspect,
    quality = 0.8,
    allowsMultipleSelection = false,
  } = options;

  const [picking, setPicking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const requestPermissions = async () => {
    const { status: cameraStatus } =
      await ImagePicker.requestCameraPermissionsAsync();
    const { status: mediaStatus } =
      await ImagePicker.requestMediaLibraryPermissionsAsync();

    if (cameraStatus !== 'granted' || mediaStatus !== 'granted') {
      setError('Camera and media library permissions are required');
      return false;
    }
    return true;
  };

  const pickImage = async (): Promise<ImagePickerResult | null> => {
    try {
      setPicking(true);
      setError(null);

      const hasPermission = await requestPermissions();
      if (!hasPermission) return null;

      const result = await ImagePicker.launchImageLibraryAsync({
        mediaTypes: ImagePicker.MediaTypeOptions.Images,
        allowsEditing,
        aspect,
        quality,
        allowsMultipleSelection,
      });

      if (result.canceled) {
        return null;
      }

      const asset = result.assets[0];
      return {
        uri: asset.uri,
        width: asset.width,
        height: asset.height,
        type: asset.type,
        fileName: asset.fileName,
      };
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Failed to pick image';
      setError(message);
      return null;
    } finally {
      setPicking(false);
    }
  };

  const takePhoto = async (): Promise<ImagePickerResult | null> => {
    try {
      setPicking(true);
      setError(null);

      const hasPermission = await requestPermissions();
      if (!hasPermission) return null;

      const result = await ImagePicker.launchCameraAsync({
        allowsEditing,
        aspect,
        quality,
      });

      if (result.canceled) {
        return null;
      }

      const asset = result.assets[0];
      return {
        uri: asset.uri,
        width: asset.width,
        height: asset.height,
        type: asset.type,
        fileName: asset.fileName,
      };
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Failed to take photo';
      setError(message);
      return null;
    } finally {
      setPicking(false);
    }
  };

  return {
    pickImage,
    takePhoto,
    picking,
    error,
  };
};
