import { invoke } from '@tauri-apps/api/core';
import React, { useEffect, useRef, useState } from 'react';
import Stats from 'stats-gl';
import * as THREE from 'three';
import './App.css';

function App() {
  const [iterations, setIterations] = useState(5);
  const [parallel, setParallel] = useState(true);
  const [isGenerating, setIsGenerating] = useState(false);
  const [generationTime, setGenerationTime] = useState(0);
  
  const mountRef = useRef(null);
  const threeRef = useRef({}); // To hold three.js objects

  // Effect for initializing the 3D scene. Runs only once.
  useEffect(() => {
    const currentMount = mountRef.current;
    
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(75, currentMount.clientWidth / currentMount.clientHeight, 0.1, 1000);
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    
    renderer.setSize(currentMount.clientWidth, currentMount.clientHeight);
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setClearColor(0x111111);
    currentMount.appendChild(renderer.domElement);
    
    camera.position.z = 2;

    const stats = new Stats();
    stats.dom.style.position = 'absolute';
    stats.dom.style.top = '10px';
    stats.dom.style.left = '10px';
    currentMount.appendChild(stats.dom);

    // Store objects for access in other effects/functions
    threeRef.current = { scene, camera, renderer, stats };

    const animate = () => {
      stats.begin();
      
      const snowflake = scene.getObjectByName("snowflake");
      if (snowflake) {
        snowflake.rotation.z += 0.001;
      }
      
      renderer.render(scene, camera);
      stats.end();
      requestAnimationFrame(animate);
    };
    animate();

    const handleResize = () => {
      camera.aspect = currentMount.clientWidth / currentMount.clientHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(currentMount.clientWidth, currentMount.clientHeight);
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      if (currentMount && renderer.domElement) {
        currentMount.removeChild(renderer.domElement);
        currentMount.removeChild(stats.dom);
      }
    };
  }, []);

  const handleGenerate = async () => {
    if (isGenerating) return;
    setIsGenerating(true);
  
    const startTime = performance.now();
    try {
      const points = await invoke('generate_snowflake', { iterations: Number(iterations), parallel });
      const endTime = performance.now();
      setGenerationTime((endTime - startTime).toFixed(2));
      
      const { scene } = threeRef.current;
      if (!scene) return;

      // Clean up old snowflake
      const oldSnowflake = scene.getObjectByName("snowflake");
      if (oldSnowflake) {
          scene.remove(oldSnowflake);
          oldSnowflake.geometry.dispose();
          oldSnowflake.material.dispose();
      }
  
      const geometry = new THREE.BufferGeometry();
      const vertices = new Float32Array(points.length * 3);
      points.forEach((p, i) => {
          vertices[i * 3] = p.x;
          vertices[i * 3 + 1] = p.y;
          vertices[i * 3 + 2] = 0;
      });
  
      geometry.setAttribute('position', new THREE.BufferAttribute(vertices, 3));
      const material = new THREE.LineBasicMaterial({ color: 0x00aaff });
      const newSnowflake = new THREE.Line(geometry, material);
      newSnowflake.name = "snowflake";
      scene.add(newSnowflake);
  
    } catch (e) {
      console.error("Failed to generate snowflake:", e);
    } finally {
      setIsGenerating(false);
    }
  };

  // Effect to re-generate the snowflake when settings change.
  // useEffect(() => {
  //   // Only generate if the three.js scene has been initialized.
  //   if (threeRef.current.scene) {
  //       handleGenerate();
  //   }
  // }, [iterations, parallel]);
  
  return (
    <div className="app-container">
      <div ref={mountRef} className="render-canvas" />
      <div className="controls">
        <h1>Infinite Snowflake Generator</h1>
        <div className="control-row">
          <label htmlFor="iterations">Iterations:</label>
          <input
            type="range"
            id="iterations"
            min="0"
            max="24"
            value={iterations}
            onChange={(e) => setIterations(e.target.value)}
            disabled={isGenerating}
          />
          <span>{iterations}</span>
        </div>
        <div className="control-row toggle-row">
          <label>Parallel Calculation:</label>
          <label className="switch">
            <input type="checkbox" checked={parallel} onChange={() => setParallel(!parallel)} disabled={isGenerating} />
            <span className="slider round"></span>
          </label>
        </div>
        <button onClick={handleGenerate} disabled={isGenerating}>
          {isGenerating ? 'Generating...' : 'Re-Generate'}
        </button>
        {generationTime > 0 && (
          <p className="perf-text">
            Rust calculation time: <span>{generationTime} ms</span>
          </p>
        )}
      </div>
    </div>
  );
}

export default App;

