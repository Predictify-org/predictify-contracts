import React, { useRef } from 'react';
import { View, Text, TouchableOpacity, Image, StyleSheet } from 'react-native';
import { useCamera } from '../../hooks/useCamera';
// import { Camera } from 'expo-camera'; // Placeholder for expo-camera

interface AvatarCameraProps {
  onPictureTaken: (imageUri: string) => void;
}

export const AvatarCamera: React.FC<AvatarCameraProps> = ({ onPictureTaken }) => {
  const { hasPermission, capturedImage, takePicture, resetCapturedImage } = useCamera();
  // const cameraRef = useRef<Camera>(null); // Placeholder for camera ref

  const handleTakePicture = async () => {
    // Placeholder for actual camera integration
    // if (cameraRef.current) {
    //   const photo = await cameraRef.current.takePictureAsync();
    //   setCapturedImage(photo.uri);
    //   onPictureTaken(photo.uri);
    // }
    await takePicture(); // Call the hook's takePicture
    if (capturedImage) { // Check if capturedImage is updated by the mock takePicture
      onPictureTaken(capturedImage);
    }
  };

  const handleRetakePicture = () => {
    resetCapturedImage();
  };

  if (hasPermission === null) {
    return <View><Text>Requesting camera permission...</Text></View>;
  }
  if (hasPermission === false) {
    return <View><Text>No access to camera</Text></View>;
  }

  return (
    <View style={styles.container}>
      {capturedImage ? (
        <View style={styles.previewContainer}>
          <Image source={{ uri: capturedImage }} style={styles.previewImage} />
          <TouchableOpacity style={styles.button} onPress={handleRetakePicture}>
            <Text style={styles.text}>Retake Picture</Text>
          </TouchableOpacity>
          {/* A button to confirm and save the picture could be added here */}
        </View>
      ) : (
        <>
          {/* Placeholder for actual camera component */}
          <View style={styles.cameraPlaceholder}>
            <Text style={styles.cameraPlaceholderText}>Camera View</Text>
            {/* <Camera style={StyleSheet.absoluteFillObject} ref={cameraRef} /> */}
          </View>
          <TouchableOpacity style={styles.button} onPress={handleTakePicture}>
            <Text style={styles.text}>Take Picture</Text>
          </TouchableOpacity>
        </>
      )}
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    backgroundColor: '#000',
  },
  cameraPlaceholder: {
    width: '80%',
    height: '60%',
    backgroundColor: '#333',
    justifyContent: 'center',
    alignItems: 'center',
    borderRadius: 10,
    marginBottom: 20,
  },
  cameraPlaceholderText: {
    color: '#fff',
    fontSize: 18,
  },
  button: {
    backgroundColor: '#007AFF',
    padding: 15,
    borderRadius: 5,
    marginVertical: 10,
  },
  text: {
    color: 'white',
    fontSize: 18,
    textAlign: 'center',
  },
  previewContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  previewImage: {
    width: 300,
    height: 300,
    borderRadius: 150,
    marginBottom: 20,
  },
});
