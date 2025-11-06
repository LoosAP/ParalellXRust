import { OrbitControls } from '@react-three/drei';
import { Canvas, useFrame, useLoader } from '@react-three/fiber';
import { Suspense, useRef, useState } from 'react';
import * as three from 'three';

function Background() {
  const texture = useLoader(three.TextureLoader, 'https://raw.githubusercontent.com/mrdoob/three.js/master/examples/textures/planets/earth_atmos_2048.jpg');
  return (
    <mesh scale={10}>
      <sphereGeometry args={[1, 64, 64]} />
      <meshBasicMaterial map={texture} side={three.BackSide} />
    </mesh>
  );
}

function Water() {
  const ref = useRef();
  const normalMap = useLoader(three.TextureLoader, 'water_normal.jpg');
  const [fbo] = useState(() => new three.WebGLRenderTarget(1024, 1024));

  normalMap.wrapS = normalMap.wrapT = three.RepeatWrapping;

  useFrame(({ gl, scene, camera, clock }) => {
    ref.current.visible = false;
    gl.setRenderTarget(fbo);
    gl.render(scene, camera);
    ref.current.visible = true;
    gl.setRenderTarget(null);

    ref.current.material.uniforms.u_time.value = clock.getElapsedTime();
  });

  return (
    <mesh ref={ref} rotation={[-Math.PI / 2, 0, 0]}>
      <sphereGeometry args={[4, 64, 64]} />
      <shaderMaterial
        uniforms={{
          u_texture: { value: fbo.texture },
          u_normal_map: { value: normalMap },
          u_time: { value: 0.0 },
          u_refraction_strength: { value: 0.05 },
          u_water_color: { value: new three.Color('#88c5e1') },
        }}
        vertexShader={`
          varying vec2 vUv;
          varying vec4 v_screen_position;
          
          void main() {
            vUv = uv;
            // Project the vertex position to screen space
            v_screen_position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
            gl_Position = v_screen_position;
          }
        `}
        fragmentShader={`
          uniform sampler2D u_texture;
          uniform sampler2D u_normal_map;
          uniform float u_time;
          uniform float u_refraction_strength;
          uniform vec3 u_water_color;
          
          varying vec2 vUv;
          varying vec4 v_screen_position;
          
          void main() {
            // Animate the normal map's UV coordinates to simulate flowing water
            vec2 animated_uv = vUv + u_time * 0.05;
            
            // Look up the normal vector from our ripple texture.
            // The .rgb values are mapped to x, y, z directions.
            vec3 normal = texture2D(u_normal_map, animated_uv).rgb * 2.0 - 1.0;
            
            // Calculate the screen-space UVs. We divide by v_screen_position.w to get normalized coordinates.
            vec2 screen_uv = v_screen_position.xy / v_screen_position.w;
            
            // Add the ripple distortion to the screen UVs
            vec2 refracted_uv = screen_uv + normal.xy * u_refraction_strength;
            
            // Look up the color of the background at the distorted coordinate
            vec3 refracted_color = texture2D(u_texture, refracted_uv).rgb;
            
            // Mix the refracted background color with a base water color for the final effect
            gl_FragColor = vec4(mix(refracted_color, u_water_color, 0.3), 1.0);
          }
        `}
      />
    </mesh>
  );
}

export default function App() {
  return (
    <Canvas camera={{ position: [10, 0, 4], fov: 75 }}>
      <Suspense fallback={null}>
        <Background />
        <Water />
      </Suspense>
        <OrbitControls enablePan={false} enableZoom={false} />
    </Canvas>
  );
}