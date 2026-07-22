import {
  AxesHelper,
  Box3,
  Box3Helper,
  Color,
  DirectionalLight,
  Group,
  Material,
  Mesh,
  MeshBasicMaterial,
  Object3D,
  PerspectiveCamera,
  Scene,
  SphereGeometry,
  Vector3,
  WebGLRenderer,
} from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { TransformControls } from "three/addons/controls/TransformControls.js";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";

import type { AssetRecord, ConnectorRecord } from "../types";

export interface AssetViewer {
  clear(): void;
  loadAsset(url: string, asset: AssetRecord): Promise<void>;
  setConnectors(connectors: ConnectorRecord[]): void;
  selectConnector(connectorId: string | null): void;
  dispose(): void;
}

export function createAssetViewer(
  container: HTMLElement,
  onConnectorMoved: (
    connectorId: string,
    position: [number, number, number],
    orientation: [number, number, number, number],
  ) => void,
): AssetViewer {
  const scene = new Scene();
  scene.background = new Color(0x111827);

  const camera = new PerspectiveCamera(55, 1, 0.01, 1000);
  camera.position.set(2.5, 2.0, 3.0);

  const renderer = new WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setSize(container.clientWidth, container.clientHeight);
  container.appendChild(renderer.domElement);

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;

  const transform = new TransformControls(camera, renderer.domElement);
  scene.add(transform.getHelper());
  transform.addEventListener("dragging-changed", (event) => {
    controls.enabled = !(event as { value: boolean }).value;
  });

  scene.add(new AxesHelper(1.5));
  const light = new DirectionalLight(0xffffff, 2.5);
  light.position.set(2, 4, 3);
  scene.add(light);

  let model: Group | null = null;
  let boundsHelper: Box3Helper | null = null;
  const markers = new Map<string, Mesh>();
  let selectedConnectorId: string | null = null;
  let disposed = false;

  const resizeObserver = new ResizeObserver(() => resize());
  resizeObserver.observe(container);

  function resize() {
    const width = Math.max(container.clientWidth, 1);
    const height = Math.max(container.clientHeight, 1);
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height);
  }

  function animate() {
    if (disposed) {
      return;
    }

    controls.update();
    renderer.render(scene, camera);
    requestAnimationFrame(animate);
  }
  animate();

  transform.addEventListener("objectChange", () => {
    if (!selectedConnectorId) {
      return;
    }

    const marker = markers.get(selectedConnectorId);
    if (!marker) {
      return;
    }

    onConnectorMoved(
      selectedConnectorId,
      [marker.position.x, marker.position.y, marker.position.z],
      [
        marker.quaternion.x,
        marker.quaternion.y,
        marker.quaternion.z,
        marker.quaternion.w,
      ],
    );
  });

  function clearModel() {
    if (model) {
      scene.remove(model);
      disposeObject(model);
      model = null;
    }

    if (boundsHelper) {
      scene.remove(boundsHelper);
      boundsHelper.geometry.dispose();
      disposeMaterial(boundsHelper.material);
      boundsHelper = null;
    }
  }

  function clearMarkers() {
    transform.detach();
    for (const marker of markers.values()) {
      scene.remove(marker);
      marker.geometry.dispose();
      disposeMaterial(marker.material);
    }
    markers.clear();
  }

  const viewer: AssetViewer = {
    clear() {
      clearModel();
      clearMarkers();
      selectedConnectorId = null;
    },
    async loadAsset(url, asset) {
      clearModel();
      clearMarkers();

      const gltf = await new GLTFLoader().loadAsync(url);
      model = gltf.scene;
      scene.add(model);

      const bounds = new Box3(
        new Vector3(...asset.bounds.min),
        new Vector3(...asset.bounds.max),
      );
      boundsHelper = new Box3Helper(bounds, 0x38bdf8);
      scene.add(boundsHelper);

      const size = new Vector3();
      bounds.getSize(size);
      const center = new Vector3();
      bounds.getCenter(center);
      const radius = Math.max(size.length(), 1);
      controls.target.copy(center);
      camera.position.copy(center.clone().add(new Vector3(radius, radius, radius)));
      controls.update();
      resize();
    },
    setConnectors(connectors) {
      const nextConnectorIds = new Set(
        connectors
          .filter((connector) => connector.frame.kind === "frame3d")
          .map((connector) => connector.connector_id),
      );

      for (const [connectorId, marker] of markers) {
        if (nextConnectorIds.has(connectorId)) {
          continue;
        }

        scene.remove(marker);
        marker.geometry.dispose();
        disposeMaterial(marker.material);
        markers.delete(connectorId);
      }

      for (const connector of connectors) {
        if (connector.frame.kind !== "frame3d") {
          continue;
        }

        let marker = markers.get(connector.connector_id);
        if (!marker) {
          marker = new Mesh(
            new SphereGeometry(0.045, 16, 16),
            new MeshBasicMaterial(),
          );
          marker.name = connector.connector_id;
          markers.set(connector.connector_id, marker);
          scene.add(marker);
        }

        marker.position.set(...connector.frame.position);
        marker.quaternion.set(...connector.frame.orientation_quat_xyzw);
        (marker.material as MeshBasicMaterial).color.setHex(
          connector.class ? 0x22c55e : 0xf59e0b,
        );
      }

      viewer.selectConnector(selectedConnectorId);
    },
    selectConnector(connectorId) {
      selectedConnectorId = connectorId;
      const marker = connectorId ? markers.get(connectorId) : undefined;
      if (marker) {
        transform.attach(marker);
      } else {
        transform.detach();
      }
    },
    dispose() {
      disposed = true;
      resizeObserver.disconnect();
      viewer.clear();
      transform.dispose();
      controls.dispose();
      renderer.dispose();
      renderer.domElement.remove();
    },
  };

  return viewer;
}

function disposeObject(object: Object3D) {
  object.traverse((child) => {
    const mesh = child as Mesh;
    if (!mesh.isMesh) {
      return;
    }

    mesh.geometry.dispose();
    disposeMaterial(mesh.material);
  });
}

function disposeMaterial(material: Material | Material[]) {
  if (Array.isArray(material)) {
    for (const item of material) {
      item.dispose();
    }
    return;
  }

  material.dispose();
}
