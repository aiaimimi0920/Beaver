import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { OBJLoader } from "three/examples/jsm/loaders/OBJLoader.js";
export function ModelPreview({
  url,
  fit = false,
}: {
  url: string;
  fit?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [error, setError] = useState("");
  const [loadedModel, setLoadedModel] = useState(false);
  useEffect(() => {
    const host = ref.current!;
    let dead = false;
    let model: THREE.Object3D | undefined;
    setError("");
    setLoadedModel(false);
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true });
    } catch {
      setError("当前设备无法创建 WebGL 预览，可将模型交给 Codex 处理。");
      return;
    }
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    host.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    scene.background = new THREE.Color("#090c11");
    scene.add(new THREE.HemisphereLight(0xffffff, 0x444455, 3));
    const light = new THREE.DirectionalLight(0xffffff, 3);
    light.position.set(3, 5, 4);
    scene.add(light);
    const camera = new THREE.PerspectiveCamera(45, 1, 0.01, 10000);
    camera.position.set(3, 2, 3);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    const resize = () => {
      const height = fit ? Math.max(1, host.clientHeight) : 360;
      renderer.setSize(host.clientWidth, height);
      camera.aspect = host.clientWidth / height;
      camera.updateProjectionMatrix();
    };
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(host);
    const loaded = (object: THREE.Object3D) => {
      if (dead) {
        dispose(object);
        return;
      }
      model = object;
      scene.add(object);
      const box = new THREE.Box3().setFromObject(object);
      const center = box.getCenter(new THREE.Vector3());
      const extent = box.getSize(new THREE.Vector3()).length() || 1;
      controls.target.copy(center);
      camera.position
        .copy(center)
        .add(new THREE.Vector3(extent, extent * 0.6, extent));
      camera.far = extent * 100 + 100;
      camera.updateProjectionMatrix();
      controls.update();
      setLoadedModel(true);
    };
    const failed = (e: unknown) => {
      if (!dead) setError(String(e));
    };
    if (new URL(url).pathname.toLowerCase().endsWith(".obj"))
      new OBJLoader().load(url, loaded, undefined, failed);
    else new GLTFLoader().load(url, (g) => loaded(g.scene), undefined, failed);
    renderer.setAnimationLoop(() => {
      controls.update();
      renderer.render(scene, camera);
    });
    return () => {
      dead = true;
      observer.disconnect();
      renderer.setAnimationLoop(null);
      controls.dispose();
      if (model) dispose(model);
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, [url, fit]);
  return (
    <>
      <div
        className="model-view"
        style={fit ? { height: "100%", minHeight: 0 } : undefined}
        aria-label="3D 模型预览"
        data-loaded={loadedModel}
        ref={ref}
      />
      {error && <div className="error-box">模型预览失败：{error}</div>}
      {!fit && (
        <small className="muted">拖动旋转 · 滚轮缩放 · glTF / GLB / OBJ</small>
      )}
    </>
  );
}
function dispose(model: THREE.Object3D) {
  model.traverse((o) => {
    if (o instanceof THREE.Mesh) {
      o.geometry.dispose();
      for (const material of Array.isArray(o.material)
        ? o.material
        : [o.material]) {
        for (const value of Object.values(material))
          if (value instanceof THREE.Texture) value.dispose();
        material.dispose();
      }
    }
  });
}
